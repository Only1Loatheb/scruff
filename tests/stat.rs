//! Runs the formatter tests in `tests/fmt/*.stat`, written in scalafmt's
//! format:
//!
//! ```text
//! maxColumn = 40          <- optional settings for the whole file
//!
//! <<< name of the case
//! maxColumn = 20          <- optional settings for this case, ended by ===
//! ===
//! input
//! >>>
//! expected output
//! ```
//!
//! Lines starting with `#` in settings are comments. `<<< SKIP name` skips
//! a case. `<<< ONLY name` runs only the marked cases,
//! then fails so it isn't committed by accident. An expected output of
//! `test does not parse` means the formatter must refuse the input.
//!
//! Each case also checks that formatting the expected output leaves it
//! unchanged.
//!
//! The dialect is scuff's default, `scala3`, unless the file or case names
//! another.

use std::path::{Path, PathBuf};

use scuff::config::Dialect;
use scuff::fmt::{FormatError, FormatOptions, format};

struct Case {
    file: PathBuf,
    /// 1-based line of the `<<<` line.
    line: usize,
    name: String,
    marker: Marker,
    options: FormatOptions,
    input: String,
    expected: String,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Marker {
    None,
    Skip,
    Only,
}

const DOES_NOT_PARSE: &str = "test does not parse";

/// Applies `key = value` settings. The formatter has almost no options, so
/// anything but `maxColumn` and `dialect` is a mistake in the test file.
fn apply_settings(options: &mut FormatOptions, lines: &[&str], file: &Path, first_line: usize) {
    for (i, line) in lines.iter().enumerate() {
        let at = format!("{}:{}", file.display(), first_line + i);
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .unwrap_or_else(|| panic!("{at}: expected `key = value`, found {line:?}"));
        match key.trim() {
            "maxColumn" => {
                options.max_width = value
                    .trim()
                    .parse()
                    .unwrap_or_else(|_| panic!("{at}: maxColumn must be a number"));
            }
            "dialect" => {
                options.dialect = Dialect::named(value.trim())
                    .unwrap_or_else(|| panic!("{at}: unknown dialect {value:?}"));
            }
            other => {
                panic!("{at}: unknown setting {other:?}; the settings are maxColumn and dialect")
            }
        }
    }
}

fn parse_file(file: &Path) -> Vec<Case> {
    let text = std::fs::read_to_string(file).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let starts: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].starts_with("<<<"))
        .collect();

    let mut defaults = FormatOptions::default();
    let header_end = starts.first().copied().unwrap_or(lines.len());
    apply_settings(&mut defaults, &lines[..header_end], file, 1);

    let mut cases = Vec::new();
    for (n, &start) in starts.iter().enumerate() {
        let end = starts.get(n + 1).copied().unwrap_or(lines.len());
        let at = format!("{}:{}", file.display(), start + 1);
        let title = lines[start].trim_start_matches('<').trim();
        let (marker, name) = match title.split_once(' ') {
            Some(("SKIP", rest)) => (Marker::Skip, rest),
            Some(("ONLY", rest)) => (Marker::Only, rest),
            _ => (Marker::None, title),
        };
        let body = &lines[start + 1..end];
        let arrow = body
            .iter()
            .position(|l| *l == ">>>")
            .unwrap_or_else(|| panic!("{at}: case {name:?} has no >>> line"));
        let (mut input, expected) = (&body[..arrow], &body[arrow + 1..]);

        let mut options = defaults.clone();
        if let Some(separator) = input.iter().position(|l| *l == "===") {
            apply_settings(&mut options, &input[..separator], file, start + 2);
            input = &input[separator + 1..];
        }
        // Blank lines between cases belong to neither.
        let expected_len = expected
            .iter()
            .rposition(|l| !l.is_empty())
            .map_or(0, |i| i + 1);
        cases.push(Case {
            file: file.to_path_buf(),
            line: start + 1,
            name: name.to_string(),
            marker,
            options,
            input: input.iter().map(|l| format!("{l}\n")).collect(),
            expected: expected[..expected_len]
                .iter()
                .map(|l| format!("{l}\n"))
                .collect(),
        });
    }
    cases
}

fn stat_files() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fmt");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "stat"))
        .collect();
    files.sort();
    files
}

/// The problem with a case, if any.
fn check(case: &Case) -> Option<String> {
    let result = format(&case.input, &case.options);
    if case.expected.trim() == DOES_NOT_PARSE {
        return match result {
            Err(FormatError::SyntaxErrors(_)) => None,
            Err(e) => Some(format!("expected a syntax error, got: {e}")),
            Ok(out) => Some(format!("expected a syntax error, got output:\n{out}")),
        };
    }
    let out = match result {
        Ok(out) => out,
        Err(e) => return Some(format!("formatting failed: {e}")),
    };
    if out != case.expected {
        return Some(format!("--- expected\n{}--- actual\n{out}", case.expected));
    }
    match format(&case.expected, &case.options) {
        Ok(again) if again == case.expected => None,
        Ok(again) => Some(format!(
            "not idempotent; formatting the expected output gives:\n{again}"
        )),
        Err(e) => Some(format!("formatting the expected output failed: {e}")),
    }
}

#[test]
fn stat_files_pass() {
    let cases: Vec<Case> = stat_files().iter().flat_map(|f| parse_file(f)).collect();
    assert!(!cases.is_empty(), "no cases in tests/fmt/*.stat");
    let only = cases.iter().any(|c| c.marker == Marker::Only);

    let (mut passed, mut skipped, mut failures) = (0, 0, Vec::new());
    for case in &cases {
        let run = if only {
            case.marker == Marker::Only
        } else {
            case.marker != Marker::Skip
        };
        if !run {
            skipped += 1;
            continue;
        }
        match check(case) {
            None => passed += 1,
            Some(problem) => failures.push(format!(
                "{}:{}: {}\n{problem}",
                case.file.display(),
                case.line,
                case.name
            )),
        }
    }
    for failure in &failures {
        eprintln!("\n{failure}");
    }
    eprintln!(
        "\n{passed} passed, {} failed, {skipped} skipped",
        failures.len()
    );
    assert!(failures.is_empty(), "{} .stat cases failed", failures.len());
    assert!(!only, "remove ONLY markers before committing");
}
