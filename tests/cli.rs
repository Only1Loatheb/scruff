//! Runs the `scuff` binary.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

const UNFORMATTED: &str = "object A {\n  def f(x:Int)=x+1\n}\n";
const FORMATTED: &str = "object A {\n  def f(x: Int) = x + 1\n}\n";

/// A file with `contents` in the test's own temporary directory.
fn file(name: &str, contents: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

fn scuff(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_scuff"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn format_overwrites_a_file() {
    let path = file("format_overwrites.scala", UNFORMATTED);
    let out = scuff(&["format", path.to_str().unwrap()], "");
    assert!(out.status.success());
    assert_eq!(text(&out.stdout), "");
    assert_eq!(text(&out.stderr), "");
    // Exactly the formatted text: no extra line break at the end.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), FORMATTED);
}

#[test]
fn format_leaves_a_formatted_file_unwritten() {
    let path = file("format_unwritten.scala", FORMATTED);
    let before = std::fs::metadata(&path).unwrap().modified().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));
    let out = scuff(&["format", path.to_str().unwrap()], "");
    assert!(out.status.success());
    let after = std::fs::metadata(&path).unwrap().modified().unwrap();
    assert_eq!(before, after);
}

#[test]
fn format_reads_standard_input_and_prints() {
    let out = scuff(&["format"], UNFORMATTED);
    assert!(out.status.success());
    // Exactly the formatted text: no extra line break at the end.
    assert_eq!(text(&out.stdout), FORMATTED);
    // Formatting its own output again changes nothing.
    let again = scuff(&["format", "-"], &text(&out.stdout));
    assert_eq!(text(&again.stdout), FORMATTED);
}

#[test]
fn check_passes_quietly_on_a_formatted_file() {
    let path = file("check_formatted.scala", FORMATTED);
    let out = scuff(&["format", "--check", path.to_str().unwrap()], "");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stdout), "");
    assert_eq!(text(&out.stderr), "");
}

#[test]
fn check_names_a_file_that_would_change() {
    let path = file("check_unformatted.scala", UNFORMATTED);
    let out = scuff(&["format", "--check", path.to_str().unwrap()], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "");
    assert_eq!(
        text(&out.stderr),
        format!("{}: would be reformatted\n", path.display())
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), UNFORMATTED);
}

#[test]
fn syntax_errors_exit_with_2_and_say_where() {
    let source = "object A {\n  def f( = 1\n";
    let path = file("syntax_error.scala", source);
    let out = scuff(&["format", path.to_str().unwrap()], "");
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(text(&out.stdout), "");
    // The file is left as it was.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
    let stderr = text(&out.stderr);
    assert!(
        stderr.starts_with(&format!("{}:2:", path.display())),
        "{stderr}"
    );
    // With --check too: an error isn't "would be reformatted".
    let check = scuff(&["format", "--check", path.to_str().unwrap()], "");
    assert_eq!(check.status.code(), Some(2));
}

#[test]
fn a_missing_file_exits_with_2() {
    let out = scuff(&["format", "no-such-file.scala"], "");
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("can't read no-such-file.scala"));
}
