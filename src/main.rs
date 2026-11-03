use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use anstyle::{AnsiColor, Style};
use clap::{Parser, Subcommand, ValueEnum};
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
    /// Ends with a count of the files changed and left unchanged.
    ///
    /// Exits with 1 if `--check` or `--diff` finds a file would change, and
    /// with 2 if one can't be formatted: it can't be read or written, has
    /// syntax errors, or formatting would change what the code means.
    #[command(visible_alias = "fmt")]
    Format {
        /// The files and directories to format, each file overwritten with
        /// the result; the current directory when omitted. `-` reads
        /// standard input and prints the result
        #[arg(default_value = ".")]
        files: Vec<PathBuf>,

        /// Write nothing; instead show how each file would change, and exit
        /// with 1 if any would
        #[arg(long)]
        check: bool,

        /// Write nothing; instead print a unified diff of each change, and
        /// exit with 1 if any file would change
        #[arg(long)]
        diff: bool,

        /// Print only the diffs: no count of files at the end, no warnings,
        /// and errors only with `--check`, as in Ruff
        #[arg(short, long)]
        quiet: bool,

        /// Print nothing but the formatted text or the diff; errors show only
        /// in the exit code
        #[arg(short, long)]
        silent: bool,

        /// When to color the output; `auto` colors it on a terminal, unless
        /// `NO_COLOR` is set
        #[arg(long, value_name = "WHEN", default_value = "auto")]
        color: Color,
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
        Command::Format {
            files,
            check,
            diff,
            quiet,
            silent,
            color,
        } => {
            // `--diff` shows more than `--check`, so it wins, as in Ruff.
            let mode = match (check, diff) {
                (_, true) => Mode::Diff,
                (true, false) => Mode::Check,
                (false, false) => Mode::Write,
            };
            let paints = Paints {
                out: Paint::new(color, &std::io::stdout()),
                err: Paint::new(color, &std::io::stderr()),
            };
            format(files, mode, Verbosity::from_flags(quiet, silent), paints)
        }
        Command::Parse { file, quiet } => parse(file, quiet),
        Command::Comments { file } => comments(file),
    }
}

/// What `format` does with a file that would change.
#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Overwrite it.
    Write,
    /// Show a short diff of it.
    Check,
    /// Print a unified diff of it.
    Diff,
}

/// When to color the output.
#[derive(Clone, Copy, ValueEnum)]
enum Color {
    /// On a terminal, unless `NO_COLOR` is set or `CLICOLOR_FORCE` is.
    Auto,
    Always,
    Never,
}

const LOCATION: Style = Style::new().bold();
const ERROR: Style = AnsiColor::BrightRed.on_default().bold();
const WARNING: Style = AnsiColor::BrightYellow.on_default().bold();
const GUTTER: Style = AnsiColor::BrightBlue.on_default().bold();
const DELETED: Style = AnsiColor::Red.on_default();
const INSERTED: Style = AnsiColor::Green.on_default();

/// Whether to color what goes to one stream, and the coloring.
#[derive(Clone, Copy)]
struct Paint {
    on: bool,
}

impl Paint {
    /// Decided once, for `stream`: Formatted text and the text of a diff go
    /// out exactly as they are, so nothing is stripped on the way.
    fn new(color: Color, stream: &impl anstream::stream::RawStream) -> Paint {
        let on = match color {
            Color::Always => true,
            Color::Never => false,
            Color::Auto => anstream::AutoStream::choice(stream) != anstream::ColorChoice::Never,
        };
        Paint { on }
    }

    /// `text` in `style`, when coloring.
    fn paint(self, style: Style, text: &str) -> String {
        if self.on {
            format!("{style}{text}{style:#}")
        } else {
            text.to_string()
        }
    }

    /// `error: {message}`, as a line.
    fn error(self, message: &str) -> String {
        format!("{} {message}\n", self.paint(ERROR, "error:"))
    }

    /// `{place}: error: {message}`, as a line.
    fn error_at(self, place: &str, message: &str) -> String {
        format!("{}: {}", self.paint(LOCATION, place), self.error(message))
    }
}

/// How to color standard output and standard error.
#[derive(Clone, Copy)]
struct Paints {
    out: Paint,
    err: Paint,
}

/// How much `format` prints besides the formatted text or the diff.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum Verbosity {
    /// Nothing; errors show only in the exit code.
    Silent,
    /// No count of files and no warnings, and errors only where they're
    /// part of `--check`'s report, as in Ruff.
    Quiet,
    Normal,
}

impl Verbosity {
    fn from_flags(quiet: bool, silent: bool) -> Verbosity {
        if silent {
            Verbosity::Silent
        } else if quiet {
            Verbosity::Quiet
        } else {
            Verbosity::Normal
        }
    }

    /// Whether errors are printed. Ruff's `--quiet` keeps them only in
    /// `--check`'s report on files, not on standard input.
    fn shows_errors(self, mode: Mode, stdin: bool) -> bool {
        match self {
            Verbosity::Normal => true,
            Verbosity::Quiet => mode == Mode::Check && !stdin,
            Verbosity::Silent => false,
        }
    }
}

fn format(files: Vec<PathBuf>, mode: Mode, verbosity: Verbosity, paints: Paints) -> ExitCode {
    let stdin = |f: &PathBuf| f.as_os_str() == "-";
    if files.len() == 1 && stdin(&files[0]) {
        return format_stdin(mode, verbosity, paints);
    }
    let shows_errors = verbosity.shows_errors(mode, false);
    let error = |message: &str| {
        if shows_errors {
            eprint!("{}", paints.err.error(message));
        }
    };
    if files.iter().any(stdin) {
        error("`-` reads standard input, so it can't come with files");
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
        error(message);
        code = 2;
    }
    // A configuration that can't be read is reported once, and its files
    // are left alone.
    let mut reported = HashSet::new();
    for file in &files {
        if let Some((path, Err(message))) = configs.for_file(file)
            && reported.insert(path)
        {
            error(message);
            code = 2;
        }
    }

    // Formatted in parallel; reported in the order given.
    let outcomes: Vec<Outcome> = files
        .par_iter()
        .map(|file| match configs.for_file(file) {
            Some((_, Err(_))) => Outcome::failed(String::new()),
            Some((_, Ok(config))) if !config.includes(file) => Outcome::skipped(),
            Some((_, Ok(config))) => format_file(file, Some(config), mode, paints),
            None => format_file(file, None, mode, paints),
        })
        .collect();
    let (mut changed, mut unchanged, mut failed) = (0, 0, 0);
    for outcome in outcomes {
        match outcome.status {
            Status::Unchanged => unchanged += 1,
            Status::Changed => changed += 1,
            Status::Failed => failed += 1,
            Status::Skipped => {}
        }
        if shows_errors {
            eprint!("{}", outcome.messages);
        }
        // A diff is what `--diff` asks for, so even `--silent` prints it.
        if mode == Mode::Diff || verbosity > Verbosity::Silent {
            print!("{}", outcome.output);
        }
        code = code.max(outcome.code(mode));
    }

    if verbosity == Verbosity::Normal {
        if changed + unchanged + failed == 0 {
            eprintln!(
                "{} no files to format under the given paths",
                paints.err.paint(WARNING, "warning:")
            );
        } else if let Some(summary) = summary(mode, changed, unchanged) {
            // Beside a diff, on standard error, so the diff can be piped.
            if mode == Mode::Diff {
                eprintln!("{summary}");
            } else {
                println!("{summary}");
            }
        }
    }
    ExitCode::from(code)
}

/// The count of files at the end, as Ruff words it: "2 files reformatted, 1
/// file left unchanged". `None` when every file failed.
fn summary(mode: Mode, changed: usize, unchanged: usize) -> Option<String> {
    let files = |n: usize| format!("{n} file{}", if n == 1 { "" } else { "s" });
    let (changed_words, unchanged_words) = match mode {
        Mode::Write => ("reformatted", "left unchanged"),
        Mode::Check | Mode::Diff => ("would be reformatted", "already formatted"),
    };
    let mut parts = Vec::new();
    if changed > 0 {
        parts.push(format!("{} {changed_words}", files(changed)));
    }
    if unchanged > 0 {
        parts.push(format!("{} {unchanged_words}", files(unchanged)));
    }
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// What became of a file.
#[derive(Clone, Copy, PartialEq)]
enum Status {
    /// Formatting changes nothing.
    Unchanged,
    /// Formatting changes it: it was written, or would be.
    Changed,
    /// It couldn't be formatted.
    Failed,
    /// Its configuration leaves it out.
    Skipped,
}

/// What formatting a file came to: what became of it, what to print to
/// standard output (a diff), and what to print to standard error.
struct Outcome {
    status: Status,
    output: String,
    messages: String,
}

impl Outcome {
    fn new(status: Status) -> Outcome {
        Outcome {
            status,
            output: String::new(),
            messages: String::new(),
        }
    }

    fn skipped() -> Outcome {
        Outcome::new(Status::Skipped)
    }

    /// A failure, with `messages` already ending in a line break.
    fn failed(messages: String) -> Outcome {
        Outcome {
            messages,
            ..Outcome::new(Status::Failed)
        }
    }

    /// The exit code this outcome calls for.
    fn code(&self, mode: Mode) -> u8 {
        match self.status {
            Status::Failed => 2,
            Status::Changed if mode != Mode::Write => 1,
            _ => 0,
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

fn format_file(file: &Path, config: Option<&Config>, mode: Mode, paints: Paints) -> Outcome {
    // `A.scala`, not `./A.scala`, when walking the current directory.
    let name = file.strip_prefix(".").unwrap_or(file).display().to_string();
    let source = match std::fs::read_to_string(file) {
        Ok(source) => source,
        Err(err) => return Outcome::failed(paints.err.error(&format!("can't read {name}: {err}"))),
    };
    let Some(result) = format_in_time(&source, options(config, Some(file))) else {
        return Outcome::failed(timed_out(&name, paints.err));
    };
    let formatted = match result {
        Ok(formatted) => formatted,
        Err(error) => return Outcome::failed(format_error(&name, &source, error, paints.err)),
    };
    if formatted == source {
        return Outcome::new(Status::Unchanged);
    }
    let output = match mode {
        // Only a file that changes is written, so its time stamp moves only
        // then.
        Mode::Write => match std::fs::write(file, formatted) {
            Ok(()) => String::new(),
            Err(err) => {
                return Outcome::failed(paints.err.error(&format!("can't write {name}: {err}")));
            }
        },
        Mode::Check => check_diff(&name, &source, &formatted, paints.out),
        Mode::Diff => unified_diff(&name, &source, &formatted, paints.out),
    };
    Outcome {
        output,
        ..Outcome::new(Status::Changed)
    }
}

/// Standard input to standard output, following the `.scalafmt.conf` for
/// the current directory. With `--check` or `--diff`, the diff instead.
fn format_stdin(mode: Mode, verbosity: Verbosity, paints: Paints) -> ExitCode {
    let error = |message: &str| {
        if verbosity.shows_errors(mode, true) {
            eprint!("{message}");
        }
        ExitCode::from(2)
    };
    let config = match Config::find(Path::new(".")) {
        Ok(config) => config,
        Err(message) => return error(&paints.err.error(&message.to_string())),
    };
    let (name, source) = match read_text(None) {
        Ok(input) => input,
        Err(message) => return error(&paints.err.error(&message)),
    };
    let Some(result) = format_in_time(&source, options(config.as_ref(), None)) else {
        return error(&timed_out(&name, paints.err));
    };
    let formatted = match result {
        Ok(formatted) => formatted,
        Err(message) => return error(&format_error(&name, &source, message, paints.err)),
    };
    match mode {
        Mode::Write => {
            print!("{formatted}");
            ExitCode::SUCCESS
        }
        _ if formatted == source => ExitCode::SUCCESS,
        Mode::Check => {
            if verbosity > Verbosity::Silent {
                print!("{}", check_diff(&name, &source, &formatted, paints.out));
            }
            ExitCode::FAILURE
        }
        Mode::Diff => {
            print!("{}", unified_diff(&name, &source, &formatted, paints.out));
            ExitCode::FAILURE
        }
    }
}

/// A unified diff from `source` to `formatted`, with a blank line after it.
fn unified_diff(name: &str, source: &str, formatted: &str, paint: Paint) -> String {
    use similar::ChangeTag;

    let diff = similar::TextDiff::from_lines(source, formatted);
    let mut out = format!(
        "--- {}\n+++ {}\n",
        paint.paint(DELETED, name),
        paint.paint(INSERTED, name)
    );
    for hunk in diff.unified_diff().iter_hunks() {
        out += &format!("{}\n", hunk.header());
        for change in hunk.iter_changes() {
            let value = change.value();
            let text = value.strip_suffix('\n').unwrap_or(value);
            match change.tag() {
                ChangeTag::Equal => out += &format!(" {text}"),
                ChangeTag::Delete => out += &paint.paint(DELETED, &format!("-{text}")),
                ChangeTag::Insert => out += &paint.paint(INSERTED, &format!("+{text}")),
            }
            out.push('\n');
            if !value.ends_with('\n') {
                out += "\\ No newline at end of file\n";
            }
        }
    }
    out + "\n"
}

/// How `--check` shows a file that would change, as Ruff does: where the
/// first change is, then each change with a line of context, numbered as
/// in the formatted text, and a blank line after it.
///
/// ```text
/// A.scala:2:10: would be reformatted
///   |
/// 1 | object A:
///   -   def f(x:Int)=x+1
/// 2 +   def f(x: Int) = x + 1
///   |
/// ```
fn check_diff(name: &str, source: &str, formatted: &str, paint: Paint) -> String {
    use similar::ChangeTag;

    let (line, col) = first_difference(source, formatted);
    let diff = similar::TextDiff::from_lines(source, formatted);
    let groups = diff.grouped_ops(1);
    let last_line = groups
        .iter()
        .filter_map(|group| group.last())
        .map(|op| op.new_range().end)
        .max()
        .unwrap_or(0);
    let width = last_line.to_string().len();
    // The empty gutter, before and after the changes.
    let rule = format!("{} {}\n", " ".repeat(width), paint.paint(GUTTER, "|"));

    let mut out = format!(
        "{}: {}\n{rule}",
        paint.paint(LOCATION, &format!("{name}:{line}:{col}")),
        paint.paint(ERROR, "would be reformatted"),
    );
    for (i, group) in groups.iter().enumerate() {
        if i > 0 {
            out += &paint.paint(GUTTER, &"-".repeat(80));
            out.push('\n');
        }
        for change in group.iter().flat_map(|op| diff.iter_changes(op)) {
            let (number, sign, style) = match change.tag() {
                ChangeTag::Equal => (change.new_index(), "|", None),
                ChangeTag::Delete => (None, "-", Some(DELETED)),
                ChangeTag::Insert => (change.new_index(), "+", Some(INSERTED)),
            };
            let number = number.map_or(String::new(), |n| (n + 1).to_string());
            let sign_style = style.map_or(GUTTER, Style::bold);
            out += &format!(
                "{} {}",
                paint.paint(GUTTER, &format!("{number:<width$}")),
                paint.paint(sign_style, sign)
            );
            let text = change.value();
            let text = text.strip_suffix('\n').unwrap_or(text);
            let text = text.strip_suffix('\r').unwrap_or(text);
            // No space after the sign when the line is empty.
            if !text.is_empty() {
                out.push(' ');
                out += &style.map_or(text.to_string(), |style| paint.paint(style, text));
            }
            out.push('\n');
        }
    }
    out + &rule + "\n"
}

/// The 1-based line and column in `source` of the first character
/// `formatted` differs in.
fn first_difference(source: &str, formatted: &str) -> (usize, usize) {
    let at = source
        .char_indices()
        .zip(formatted.chars())
        .find(|((_, a), b)| a != b)
        .map_or(source.len().min(formatted.len()), |((i, _), _)| i);
    let before = &source[..at];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (
        before.matches('\n').count() + 1,
        before[line_start..].chars().count() + 1,
    )
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
fn timed_out(name: &str, paint: Paint) -> String {
    paint.error_at(
        name,
        &format!(
            "not formatted: took longer than {} seconds",
            TIME_LIMIT.as_secs()
        ),
    )
}

/// What to print when `name` can't be formatted.
fn format_error(name: &str, source: &str, error: fmt::FormatError, paint: Paint) -> String {
    match error {
        fmt::FormatError::SyntaxErrors(errors) => {
            let tree = lossless_tree::parse(source);
            let mut out = String::new();
            for error in errors {
                let (line, col) = tree.line_col(error.range().start());
                out += &paint.error_at(&format!("{name}:{line}:{col}"), &error.to_string());
            }
            out + &format!(
                "{}: not formatted because of syntax errors\n",
                paint.paint(LOCATION, name)
            )
        }
        error => paint.error_at(name, &error.to_string()),
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
    read_text(file).map_err(|message| {
        eprintln!("error: {message}");
        ExitCode::from(2)
    })
}

/// Reads the file, or standard input when it's missing or `-`. Returns a
/// display name with the text, or the error to print.
fn read_text(file: Option<PathBuf>) -> Result<(String, String), String> {
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
    result
        .map(|source| (name.clone(), source))
        .map_err(|err| format!("can't read {name}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_counts_in_words() {
        assert_eq!(
            summary(Mode::Write, 1, 2).as_deref(),
            Some("1 file reformatted, 2 files left unchanged")
        );
        assert_eq!(
            summary(Mode::Check, 2, 1).as_deref(),
            Some("2 files would be reformatted, 1 file already formatted")
        );
        assert_eq!(
            summary(Mode::Diff, 0, 3).as_deref(),
            Some("3 files already formatted")
        );
        assert_eq!(summary(Mode::Write, 0, 0), None);
    }

    #[test]
    fn first_difference_is_a_line_and_column_in_characters() {
        assert_eq!(first_difference("val x=1\n", "val x = 1\n"), (1, 6));
        assert_eq!(first_difference("a\nλλ=1\n", "a\nλλ = 1\n"), (2, 3));
        // One is the start of the other.
        assert_eq!(first_difference("a\n\n", "a\n"), (2, 1));
        assert_eq!(first_difference("a", "a\n"), (1, 2));
    }

    #[test]
    fn check_diff_separates_changes_far_apart() {
        let source = "a=1\nb\nc\nd\ne=1\n";
        let formatted = "a = 1\nb\nc\nd\ne = 1\n";
        let rule = "-".repeat(80);
        assert_eq!(
            check_diff("A.scala", source, formatted, Paint { on: false }),
            format!(
                "A.scala:1:2: would be reformatted\n  |\n  - a=1\n1 + a = 1\n2 | b\n{rule}\n4 | d\n  - e=1\n5 + e = 1\n  |\n\n"
            )
        );
    }

    #[test]
    fn colors_are_only_added() {
        let source = "a=1\nb\n\nc=1";
        let formatted = "a = 1\nb\n\nc = 1\n";
        let (on, off) = (Paint { on: true }, Paint { on: false });
        for diff in [check_diff, unified_diff] {
            let colored = diff("A.scala", source, formatted, on);
            assert!(colored.contains('\x1b'));
            assert_eq!(
                anstream::adapter::strip_str(&colored).to_string(),
                diff("A.scala", source, formatted, off)
            );
        }
    }

    #[test]
    fn unified_diff_is_the_usual_one() {
        let source = "a=1\nb\nc=1";
        let formatted = "a = 1\nb\nc = 1\n";
        let usual = similar::TextDiff::from_lines(source, formatted)
            .unified_diff()
            .header("A.scala", "A.scala")
            .to_string();
        assert_eq!(
            unified_diff("A.scala", source, formatted, Paint { on: false }),
            usual + "\n"
        );
    }

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
