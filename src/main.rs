use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use clap::{Parser, Subcommand};
use rayon::prelude::*;
use scuff::config::{Config, Locator};
use scuff::fmt;
use scuff::lossless_tree;

/// Tools for working with Scala source code.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Format Scala files in place, or standard input to standard output
    ///
    /// A directory stands for the files in it and below it, except hidden
    /// ones and what `.gitignore` leaves out.
    ///
    /// Each file follows the nearest `.scuff.toml` or `.scalafmt.conf` in
    /// its directory or above, `.scuff.toml` first: files its exclude paths
    /// leave out are skipped, and lines are kept within its maximum column.
    ///
    /// Exits with 1 if `--check` finds a file would change, and with 2 if
    /// one can't be formatted: it can't be read or written, has syntax
    /// errors, or formatting would change what the code means.
    #[command(visible_alias = "fmt")]
    Format {
        /// The files and directories to format, each file overwritten with
        /// the result; reads standard input and prints the result when
        /// omitted or `-`
        files: Vec<PathBuf>,

        /// Print nothing; instead exit with 1, naming each file that would
        /// change
        #[arg(long)]
        check: bool,
    },
    /// Parse a Scala file and print its concrete syntax tree
    #[command(hide = true)]
    Parse {
        /// The file to parse; reads standard input when omitted or `-`
        file: Option<PathBuf>,

        /// Only report syntax errors; don't print the tree
        #[arg(short, long)]
        quiet: bool,
    },
    /// List each comment and the token it's attached to
    #[command(hide = true)]
    Comments {
        /// The file to read; reads standard input when omitted or `-`
        file: Option<PathBuf>,
    },
}

/// Formatting is recursive, so deep nesting needs a deep stack: about 1,000
/// levels overflow the usual 2 MB. Untouched stack is only address space.
const STACK_SIZE: usize = 256 << 20;

/// How long a file may take to format before it's left as written. Only
/// pathological code comes near it, such as calls nested thousands deep.
const TIME_LIMIT: Duration = Duration::from_secs(3);

fn main() -> ExitCode {
    let cli = Cli::parse();
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || run(cli))
        .expect("can start a thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

fn run(cli: Cli) -> ExitCode {
    match cli.command {
        Command::Format { files, check } => format(files, check),
        Command::Parse { file, quiet } => parse(file, quiet),
        Command::Comments { file } => comments(file),
    }
}

fn format(files: Vec<PathBuf>, check: bool) -> ExitCode {
    let stdin = |f: &PathBuf| f.as_os_str() == "-";
    if files.is_empty() || files.len() == 1 && stdin(&files[0]) {
        return format_stdin(check);
    }
    if files.iter().any(stdin) {
        eprintln!("error: `-` reads standard input, so it can't come with files");
        return ExitCode::from(2);
    }

    let mut code = 0;
    let Collected {
        files,
        configs,
        messages,
    } = collect_files(&files);
    // What the walks couldn't read; the rest is still formatted.
    for message in &messages {
        eprintln!("error: {message}");
        code = 2;
    }
    // A configuration that can't be read is reported once, and its files
    // are left alone.
    let mut reported = HashSet::new();
    for file in &files {
        if let Some((path, Err(error))) = configs.for_file(file)
            && reported.insert(path)
        {
            eprintln!("error: {error}");
            code = 2;
        }
    }

    // Formatted in parallel; reported in the order given.
    let outcomes: Vec<Outcome> = files
        .par_iter()
        .map(|file| match configs.for_file(file) {
            Some((_, Err(_))) => Outcome {
                code: 2,
                messages: String::new(),
            },
            Some((_, Ok(config))) if !config.includes(file) => Outcome::default(),
            Some((_, Ok(config))) => format_file(file, Some(config), check),
            None => format_file(file, None, check),
        })
        .collect();
    for outcome in outcomes {
        eprint!("{}", outcome.messages);
        code = code.max(outcome.code);
    }
    ExitCode::from(code)
}

/// What formatting a file came to: its exit code, and what to print to
/// standard error.
#[derive(Default)]
struct Outcome {
    code: u8,
    messages: String,
}

impl Outcome {
    fn error(message: String) -> Outcome {
        Outcome {
            code: 2,
            messages: message + "\n",
        }
    }
}

/// Each directory's configuration file, found once, and each file read
/// once: many directories share one.
#[derive(Default)]
struct Configs {
    locator: Locator,
    found: HashMap<PathBuf, Option<PathBuf>>,
    loaded: HashMap<PathBuf, Result<Config, String>>,
}

impl Configs {
    /// Finds and reads the configuration for files in `dir`, if it's new.
    fn load(&mut self, dir: &Path) {
        if self.found.contains_key(dir) {
            return;
        }
        let path = self.locator.locate(dir);
        if let Some(path) = &path {
            self.loaded
                .entry(path.clone())
                .or_insert_with_key(|path| Config::load(path).map_err(|e| e.to_string()));
        }
        self.found.insert(dir.to_path_buf(), path);
    }

    /// The configuration for files in `dir`, with its path, once loaded.
    fn for_dir(&self, dir: &Path) -> Option<(&Path, &Result<Config, String>)> {
        let path = self.found.get(dir)?.as_deref()?;
        Some((path, &self.loaded[path]))
    }

    fn for_file(&self, file: &Path) -> Option<(&Path, &Result<Config, String>)> {
        self.for_dir(&directory_of(file))
    }
}

/// The files to format, in order, each once: the files named, and the ones
/// found in the directories named.
struct Collected {
    files: Vec<PathBuf>,
    configs: Configs,
    /// What couldn't be read along the way.
    messages: Vec<String>,
}

/// Each file named, and the files in each directory named, with their
/// configurations. A directory's files are those its configuration
/// includes, or Scala files when it has none; hidden files and what
/// `.gitignore` leaves out are skipped, and so are directories the
/// configuration excludes whole.
fn collect_files(args: &[PathBuf]) -> Collected {
    let configs = Arc::new(Mutex::new(Configs::default()));
    let mut files = Vec::new();
    let mut messages = Vec::new();
    for arg in args {
        if !arg.is_dir() {
            files.push(arg.clone());
            continue;
        }
        let prune = Arc::clone(&configs);
        let walk = ignore::WalkBuilder::new(arg)
            .sort_by_file_name(|a, b| a.cmp(b))
            .filter_entry(move |entry| {
                if entry.depth() == 0 || !entry.file_type().is_some_and(|t| t.is_dir()) {
                    return true;
                }
                let mut configs = prune.lock().unwrap();
                configs.load(entry.path());
                !matches!(
                    configs.for_dir(entry.path()),
                    Some((_, Ok(config))) if config.excludes_dir(entry.path())
                )
            })
            .build();
        for entry in walk {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    messages.push(error.to_string());
                    continue;
                }
            };
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let file = entry.into_path();
            let mut configs = configs.lock().unwrap();
            configs.load(&directory_of(&file));
            let included = match configs.for_file(&file) {
                Some((_, Ok(config))) => config.includes(&file),
                _ => Config::included_by_default(&file),
            };
            if included {
                files.push(file);
            }
        }
    }

    // Each file once, so no two threads write one file, even through a
    // symbolic link.
    let mut seen = HashSet::new();
    files.retain(|file| {
        let key = std::fs::canonicalize(file)
            .or_else(|_| std::path::absolute(file))
            .unwrap_or_else(|_| file.clone());
        seen.insert(key)
    });
    let mut configs = Arc::into_inner(configs)
        .expect("the walks are done")
        .into_inner()
        .unwrap();
    for file in &files {
        configs.load(&directory_of(file));
    }
    Collected {
        files,
        configs,
        messages,
    }
}

/// The directory a file's configuration is looked up from.
fn directory_of(file: &Path) -> PathBuf {
    match file.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// The options for `file` under `config`, or scuff's own without one. With
/// no file, as for standard input, the dialect is the one no `fileOverride`
/// pattern changes.
fn options(config: Option<&Config>, file: Option<&Path>) -> fmt::FormatOptions {
    match config {
        Some(config) => fmt::FormatOptions {
            max_width: config.max_column,
            dialect: config.dialect_for(file),
        },
        None => fmt::FormatOptions::default(),
    }
}

fn format_file(file: &Path, config: Option<&Config>, check: bool) -> Outcome {
    let name = file.display().to_string();
    let source = match std::fs::read_to_string(file) {
        Ok(source) => source,
        Err(err) => return Outcome::error(format!("error: can't read {name}: {err}")),
    };
    let Some(result) = format_in_time(&source, options(config, Some(file))) else {
        return Outcome {
            code: 2,
            messages: timed_out(&name),
        };
    };
    match result {
        Ok(formatted) if formatted == source => Outcome::default(),
        Ok(_) if check => Outcome {
            code: 1,
            messages: format!("{name}: would be reformatted\n"),
        },
        // Only a file that changes is written, so its time stamp moves only
        // then.
        Ok(formatted) => match std::fs::write(file, formatted) {
            Ok(()) => Outcome::default(),
            Err(err) => Outcome::error(format!("error: can't write {name}: {err}")),
        },
        Err(error) => Outcome {
            code: 2,
            messages: format_error(&name, &source, error),
        },
    }
}

/// Standard input to standard output, following the `.scalafmt.conf` for
/// the current directory.
fn format_stdin(check: bool) -> ExitCode {
    let config = match Config::find(Path::new(".")) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    };
    let (name, source) = match read_source(None) {
        Ok(input) => input,
        Err(code) => return code,
    };
    let Some(result) = format_in_time(&source, options(config.as_ref(), None)) else {
        eprint!("{}", timed_out(&name));
        return ExitCode::from(2);
    };
    match result {
        Ok(formatted) if check => {
            if formatted == source {
                ExitCode::SUCCESS
            } else {
                eprintln!("{name}: would be reformatted");
                ExitCode::FAILURE
            }
        }
        Ok(formatted) => {
            print!("{formatted}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprint!("{}", format_error(&name, &source, error));
            ExitCode::from(2)
        }
    }
}

/// `f`'s result, run on a thread of its own with a deep stack, or `None` if
/// it takes longer than `limit`. A thread can't be stopped, so then it runs
/// on unwatched until the process exits.
fn within<T: Send + 'static>(limit: Duration, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (send, receive) = mpsc::channel();
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            // The receiver is gone if it gave up waiting.
            let _ = send.send(f());
        })
        .expect("can start a thread");
    receive.recv_timeout(limit).ok()
}

/// `fmt::format`, given [`TIME_LIMIT`]: `None` if it takes longer.
fn format_in_time(
    source: &str,
    options: fmt::FormatOptions,
) -> Option<Result<String, fmt::FormatError>> {
    let source = source.to_string();
    within(TIME_LIMIT, move || fmt::format(&source, &options))
}

/// What to print when formatting `name` takes too long.
fn timed_out(name: &str) -> String {
    format!(
        "{name}: error: not formatted: took longer than {} seconds\n",
        TIME_LIMIT.as_secs()
    )
}

/// What to print when `name` can't be formatted.
fn format_error(name: &str, source: &str, error: fmt::FormatError) -> String {
    match error {
        fmt::FormatError::SyntaxErrors(errors) => {
            let tree = lossless_tree::parse(source);
            let mut out = String::new();
            for error in errors {
                let (line, col) = tree.line_col(error.range().start());
                out += &format!("{name}:{line}:{col}: error: {error}\n");
            }
            out + &format!("{name}: not formatted because of syntax errors\n")
        }
        error => format!("{name}: error: {error}\n"),
    }
}

fn parse(file: Option<PathBuf>, quiet: bool) -> ExitCode {
    let (name, source) = match read_source(file) {
        Ok(input) => input,
        Err(code) => return code,
    };

    let tree = lossless_tree::parse(&source);
    if !quiet {
        print!("{:#?}", tree.syntax());
    }

    let errors = tree.errors();
    for error in errors {
        let (line, col) = tree.line_col(error.range().start());
        eprintln!("{name}:{line}:{col}: error: {error}");
    }
    if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Prints one line per comment: where it is, whether it leads or trails its
/// token (or sits at the end of the file), the token, and the comment.
fn comments(file: Option<PathBuf>) -> ExitCode {
    let (name, source) = match read_source(file) {
        Ok(input) => input,
        Err(code) => return code,
    };
    let tree = lossless_tree::parse(&source);
    let print = |trivia: lossless_tree::SyntaxTrivia<'_>, place: &str, owner: &str| {
        let (line, col) = tree.line_col(trivia.text_range().start());
        println!(
            "{name}:{line}:{col}: {place:<8} {owner:<12} {:?}",
            trivia.text()
        );
    };
    for token in tree.syntax().tokens() {
        let owner = format!("{:?}", token.text());
        for trivia in token.leading_trivia().filter(|t| t.is_comment()) {
            print(trivia, "leading", &owner);
        }
        for trivia in token.trailing_trivia().filter(|t| t.is_comment()) {
            print(trivia, "trailing", &owner);
        }
    }
    for trivia in tree.eof_trivia().filter(|t| t.is_comment()) {
        print(trivia, "eof", "");
    }
    ExitCode::SUCCESS
}

/// Reads the file, or standard input when it's missing or `-`. Returns a
/// display name with the text, or prints the error and returns exit code 2.
fn read_source(file: Option<PathBuf>) -> Result<(String, String), ExitCode> {
    let file = file.filter(|f| f.as_os_str() != "-");
    let name = file
        .as_ref()
        .map_or("<stdin>".to_string(), |f| f.display().to_string());
    let result = match &file {
        Some(path) => std::fs::read_to_string(path),
        None => {
            let mut source = String::new();
            std::io::stdin().read_to_string(&mut source).map(|_| source)
        }
    };
    result.map(|source| (name.clone(), source)).map_err(|err| {
        eprintln!("error: can't read {name}: {err}");
        ExitCode::from(2)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn within_gives_up_after_its_limit() {
        assert_eq!(within(Duration::from_secs(10), || 1), Some(1));
        let slow = || std::thread::sleep(Duration::from_secs(10));
        assert_eq!(within(Duration::from_millis(10), slow), None);
    }

    #[test]
    fn within_has_a_deep_stack() {
        fn depth(n: u64) -> u64 {
            // Enough in each frame that a 2 MB stack overflows.
            let frame = std::hint::black_box([n; 64]);
            if n == 0 {
                0
            } else {
                1 + depth(n - 1) + frame[0] - n
            }
        }
        assert_eq!(
            within(Duration::from_secs(10), || depth(20_000)),
            Some(20_000)
        );
    }
}
