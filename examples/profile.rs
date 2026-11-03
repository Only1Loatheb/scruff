//! Profiles formatting and writes a flamegraph.
//!
//! ```sh
//! cargo run --profile profiling --example profile -- [--runs N] [--out FILE] PATH...
//! ```
//!
//! Each path is a Scala file or a directory searched for them, following
//! `.gitignore` and the nearest `.scruff.toml` or `.scalafmt.conf` as `scruff
//! format` does. The files are formatted `--runs` times (default 1) on one
//! thread, so the stacks read cleanly, with a deep stack as in `scruff
//! format`, and the samples are written as an SVG flamegraph to `--out`
//! (default `flamegraph.svg`).
//!
//! pprof only works on Unix, so elsewhere this just says so.

#[cfg(not(unix))]
fn main() {
    eprintln!("profile needs Linux or macOS (pprof is Unix-only)");
    std::process::exit(1);
}

#[cfg(unix)]
use std::collections::HashMap;
#[cfg(unix)]
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Instant;

#[cfg(unix)]
use scruff::config::{Config, Locator};
#[cfg(unix)]
use scruff::fmt::{self, FormatOptions};

/// Samples per second.
#[cfg(unix)]
const FREQUENCY: i32 = 999;

/// Formatting is recursive, so deep nesting needs a deep stack, as in
/// `scruff format`.
#[cfg(unix)]
const STACK_SIZE: usize = 256 << 20;

#[cfg(unix)]
fn main() {
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(profile)
        .expect("start a thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
}

#[cfg(unix)]
fn profile() {
    let mut runs = 1;
    let mut out = PathBuf::from("flamegraph.svg");
    let mut paths = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--runs" => runs = args.next().and_then(|n| n.parse().ok()).expect("--runs N"),
            "--out" => out = args.next().expect("--out FILE").into(),
            _ => paths.push(PathBuf::from(arg)),
        }
    }
    assert!(
        !paths.is_empty(),
        "usage: profile [--runs N] [--out FILE] PATH..."
    );

    let inputs = load(&paths);
    let bytes: usize = inputs.iter().map(|(_, source, _)| source.len()).sum();
    eprintln!(
        "{} files, {:.1} MB, {runs} runs",
        inputs.len(),
        bytes as f64 / 1e6
    );

    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(FREQUENCY)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .expect("start the profiler");
    let start = Instant::now();
    let mut failed = 0;
    for _ in 0..runs {
        for (_, source, options) in &inputs {
            failed += fmt::format(source, options).is_err() as usize;
        }
    }
    let elapsed = start.elapsed().as_secs_f64();
    let report = guard.report().build().expect("build the report");

    eprintln!(
        "{elapsed:.2}s, {:.2} MB/s on one thread, {} of {} formats failed",
        (bytes * runs) as f64 / 1e6 / elapsed,
        failed,
        inputs.len() * runs
    );
    let file = std::fs::File::create(&out).expect("create the flamegraph file");
    report.flamegraph(file).expect("write the flamegraph");
    eprintln!("wrote {}", out.display());
}

/// Each Scala file under `paths` with its source and the options its
/// configuration gives.
#[cfg(unix)]
fn load(paths: &[PathBuf]) -> Vec<(PathBuf, String, FormatOptions)> {
    let mut locator = Locator::default();
    let mut configs: HashMap<PathBuf, Config> = HashMap::new();
    let mut inputs = Vec::new();
    for path in paths {
        for entry in ignore::Walk::new(path).filter_map(Result::ok) {
            let file = entry.path();
            if !file.is_file() || file.extension().is_none_or(|e| e != "scala") {
                continue;
            }
            let file = std::path::absolute(file).expect("absolute path");
            let config = locator
                .locate(file.parent().expect("a file's directory"))
                .map(|path| {
                    &*configs.entry(path).or_insert_with_key(|path| {
                        Config::load(path).expect("read the configuration")
                    })
                });
            let options = match config {
                Some(config) if !config.includes(&file) => continue,
                Some(config) => options(config, &file),
                None => FormatOptions::default(),
            };
            let source = std::fs::read_to_string(&file).expect("read the file");
            inputs.push((file, source, options));
        }
    }
    inputs
}

#[cfg(unix)]
fn options(config: &Config, file: &Path) -> FormatOptions {
    FormatOptions {
        max_width: config.max_column,
        dialect: config.dialect_for(Some(file)),
    }
}
