use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

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
    /// Each file follows the nearest `.scuff.toml` or `.scalafmt.conf` in
    /// its directory or above, `.scuff.toml` first: files its exclude paths
    /// leave out are skipped, and lines are kept within its maximum column.
    ///
    /// Exits with 1 if `--check` finds a file would change, and with 2 if
    /// one can't be formatted: it can't be read or written, has syntax
    /// errors, or formatting would change what the code means.
    Format {
        /// The files to format, each overwritten with the result; reads
        /// standard input and prints the result when omitted or `-`
        files: Vec<PathBuf>,

        /// Print nothing; instead exit with 1, naming each file that would
        /// change
        #[arg(long)]
        check: bool,
    },
    /// Parse a Scala file and print its concrete syntax tree
    Parse {
        /// The file to parse; reads standard input when omitted or `-`
        file: Option<PathBuf>,

        /// Only report syntax errors; don't print the tree
        #[arg(short, long)]
        quiet: bool,
    },
    /// List each comment and the token it's attached to
    Comments {
        /// The file to read; reads standard input when omitted or `-`
        file: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
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

    // Each directory's configuration file, found once, and each file read
    // once: many directories share one.
    let mut locator = Locator::default();
    let mut found: HashMap<PathBuf, Option<PathBuf>> = HashMap::new();
    let mut configs: HashMap<PathBuf, Result<Config, String>> = HashMap::new();
    for file in &files {
        let path = found
            .entry(directory_of(file))
            .or_insert_with_key(|dir| locator.locate(dir));
        if let Some(path) = path {
            configs
                .entry(path.clone())
                .or_insert_with_key(|path| Config::load(path).map_err(|e| e.to_string()));
        }
    }

    // Formatted in parallel; reported in the order given.
    let outcomes: Vec<Outcome> = files
        .par_iter()
        .map(|file| {
            let config = found[&directory_of(file)]
                .as_ref()
                .map(|path| &configs[path]);
            match config {
                Some(Err(error)) => Outcome::error(format!("error: {error}")),
                Some(Ok(config)) if !config.includes(file) => Outcome::default(),
                Some(Ok(config)) => format_file(file, Some(config), check),
                None => format_file(file, None, check),
            }
        })
        .collect();
    let mut code = 0;
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

/// The directory a file's configuration is looked up from.
fn directory_of(file: &Path) -> PathBuf {
    match file.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// The options for files under `config`, or scuff's own without one.
fn options(config: Option<&Config>) -> fmt::FormatOptions {
    match config {
        Some(config) => fmt::FormatOptions {
            max_width: config.max_column,
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
    match fmt::format(&source, &options(config)) {
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
    match fmt::format(&source, &options(config.as_ref())) {
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
