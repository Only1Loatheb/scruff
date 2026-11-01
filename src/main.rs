use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use ouf::lossless_tree;

/// Tools for working with Scala source code.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse a Scala file and print its concrete syntax tree
    Parse {
        /// The file to parse; reads standard input when omitted or `-`
        file: Option<PathBuf>,

        /// Only report syntax errors; don't print the tree
        #[arg(short, long)]
        quiet: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Parse { file, quiet } => parse(file, quiet),
    }
}

fn parse(file: Option<PathBuf>, quiet: bool) -> ExitCode {
    let file = file.filter(|f| f.as_os_str() != "-");
    let name = file
        .as_ref()
        .map_or("<stdin>".to_string(), |f| f.display().to_string());

    let source = match read_source(file.as_ref()) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("error: can't read {name}: {err}");
            return ExitCode::from(2);
        }
    };

    let parse = lossless_tree::parse(&source);
    if !quiet {
        print!("{:#?}", parse.syntax());
    }

    let errors = parse.errors();
    for error in errors {
        let (line, col) = parse.line_col(error.range().start());
        eprintln!("{name}:{line}:{col}: error: {error}");
    }
    if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn read_source(file: Option<&PathBuf>) -> std::io::Result<String> {
    match file {
        Some(path) => std::fs::read_to_string(path),
        None => {
            let mut source = String::new();
            std::io::stdin().read_to_string(&mut source)?;
            Ok(source)
        }
    }
}
