use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use rayon::prelude::*;
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
    /// Format a Scala file in place, or standard input to standard output
    ///
    /// Exits with 1 if `--check` finds the file would change, and with 2 if
    /// it can't be formatted: it can't be read or written, has syntax
    /// errors, or formatting would change what the code means.
    Format {
        /// The files to format, overwritten with the result; reads standard
        /// input and prints the result when omitted or `-`
        files: Vec<PathBuf>,

        /// Print nothing; instead exit with 1, naming the file, if it would
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
    let codes: Vec<ExitCode> = files
        .par_iter()
        .map(|file| {
            let (name, source) = match read_source(Some(file.clone())) {
                Ok(input) => input,
                Err(code) => {
                    return code;
                }
            };
            match fmt::format(&source, &fmt::FormatOptions::default()) {
                Ok(formatted) if check => {
                    if formatted == source {
                        // eprintln!("ok {name}");
                        ()
                    } else {
                        eprintln!("{name}: would be reformatted");
                        return ExitCode::FAILURE;
                    }
                }
                Ok(formatted) => {
                    if formatted != source {
                        match std::fs::write(&file, formatted) {
                            Ok(()) => (),
                            Err(err) => {
                                eprintln!("error: can't write {name}: {err}");
                                return ExitCode::from(2);
                            }
                        }
                    }
                }
                Err(fmt::FormatError::SyntaxErrors(errors)) => {
                    let tree = lossless_tree::parse(&source);
                    for error in errors {
                        let (line, col) = tree.line_col(error.range().start());
                        eprintln!("{name}:{line}:{col}: error: {error}");
                    }
                    eprintln!("{name}: not formatted because of syntax errors");
                    return ExitCode::from(2);
                }
                Err(error) => {
                    eprintln!("{name}: error: {error}");
                    return ExitCode::from(2);
                }
            }
            ExitCode::SUCCESS
        })
        .filter(|code| *code != ExitCode::SUCCESS)
        .collect();
    if codes.is_empty() {
        ExitCode::SUCCESS
    } else {
        codes[0]
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
