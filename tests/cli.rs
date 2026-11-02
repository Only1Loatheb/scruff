//! Runs the `scuff` binary.

use std::io::Write;
use std::path::{Path, PathBuf};
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
    scuff_in(Path::new(env!("CARGO_TARGET_TMPDIR")), args, stdin)
}

/// A new, empty directory for a test's project.
fn project(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Runs scuff in `dir`.
fn scuff_in(dir: &Path, args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_scuff"))
        .current_dir(dir)
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

#[test]
fn several_files_report_in_order_and_errors_win() {
    let dir = project("several_files");
    std::fs::write(dir.join("a.scala"), UNFORMATTED).unwrap();
    std::fs::write(dir.join("b.scala"), "object B {\n  def f( = 1\n").unwrap();
    std::fs::write(dir.join("c.scala"), UNFORMATTED).unwrap();
    let out = scuff_in(
        &dir,
        &["format", "--check", "a.scala", "b.scala", "c.scala"],
        "",
    );
    assert_eq!(out.status.code(), Some(2));
    let stderr = text(&out.stderr);
    let lines: Vec<_> = stderr.lines().collect();
    assert_eq!(lines.first(), Some(&"a.scala: would be reformatted"));
    assert_eq!(lines.last(), Some(&"c.scala: would be reformatted"));
    assert!(lines[1].starts_with("b.scala:2:"), "{stderr}");
}

#[test]
fn a_dash_with_files_is_an_error() {
    let path = file("dash_with_files.scala", FORMATTED);
    let out = scuff(&["format", "-", path.to_str().unwrap()], "");
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn files_the_config_excludes_are_skipped() {
    let dir = project("excluded");
    std::fs::write(
        dir.join(".scalafmt.conf"),
        "maxColumn = 100\nproject.excludePaths = [\"glob:**/gen/**\"]\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("gen")).unwrap();
    std::fs::write(dir.join("gen/A.scala"), UNFORMATTED).unwrap();
    std::fs::write(dir.join("B.scala"), UNFORMATTED).unwrap();
    let out = scuff_in(&dir, &["format", "--check", "gen/A.scala"], "");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stderr), "");
    let out = scuff_in(&dir, &["format", "gen/A.scala", "B.scala"], "");
    assert!(out.status.success());
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).unwrap();
    assert_eq!(read("gen/A.scala"), UNFORMATTED);
    assert_eq!(read("B.scala"), FORMATTED);
}

#[test]
fn max_column_comes_from_the_config() {
    let dir = project("max_column");
    std::fs::write(dir.join(".scalafmt.conf"), "maxColumn = 20\n").unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/A.scala"), "val x = f(alpha, beta)\n").unwrap();
    let out = scuff_in(&dir, &["format", "src/A.scala"], "");
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        std::fs::read_to_string(dir.join("src/A.scala")).unwrap(),
        "val x = f(\n  alpha,\n  beta,\n)\n"
    );
    // Standard input follows the config for the current directory.
    let out = scuff_in(&dir, &["format"], "val x = f(alpha, beta)\n");
    assert_eq!(text(&out.stdout), "val x = f(\n  alpha,\n  beta,\n)\n");
}

#[test]
fn a_broken_config_exits_with_2_and_names_it() {
    let dir = project("broken_config");
    std::fs::write(dir.join(".scalafmt.conf"), "maxColumn = wide\n").unwrap();
    std::fs::write(dir.join("A.scala"), UNFORMATTED).unwrap();
    let out = scuff_in(&dir, &["format", "A.scala"], "");
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains(".scalafmt.conf: maxColumn"),
        "{}",
        text(&out.stderr)
    );
    assert_eq!(
        std::fs::read_to_string(dir.join("A.scala")).unwrap(),
        UNFORMATTED
    );
}

#[test]
fn scuff_toml_comes_before_scalafmt_conf() {
    let dir = project("scuff_toml");
    std::fs::write(dir.join(".scalafmt.conf"), "maxColumn = 100\n").unwrap();
    std::fs::write(dir.join(".scuff.toml"), "max-column = 20\n").unwrap();
    std::fs::write(dir.join("A.scala"), "val x = f(alpha, beta)\n").unwrap();
    let out = scuff_in(&dir, &["format", "A.scala"], "");
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(
        std::fs::read_to_string(dir.join("A.scala")).unwrap(),
        "val x = f(\n  alpha,\n  beta,\n)\n"
    );
}

/// A project in its own git repository, with `files` at their paths.
fn repository(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = project(name);
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    for (path, contents) in files {
        let path = dir.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
    dir
}

fn read(dir: &Path, path: &str) -> String {
    std::fs::read_to_string(dir.join(path)).unwrap()
}

#[test]
fn a_directory_stands_for_its_scala_files() {
    let dir = repository(
        "walk",
        &[
            ("build.sbt", UNFORMATTED),
            ("src/main/scala/A.scala", UNFORMATTED),
            ("src/main/java/B.java", UNFORMATTED),
            (".hidden/C.scala", UNFORMATTED),
            ("target/D.scala", UNFORMATTED),
            (".gitignore", "target/\n"),
        ],
    );
    let out = scuff_in(&dir, &["format", "--check", "."], "");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        text(&out.stderr),
        "./build.sbt: would be reformatted\n./src/main/scala/A.scala: would be reformatted\n"
    );
    let out = scuff_in(&dir, &["format", "src"], "");
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(read(&dir, "src/main/scala/A.scala"), FORMATTED);
    assert_eq!(read(&dir, "src/main/java/B.java"), UNFORMATTED);
    assert_eq!(read(&dir, "build.sbt"), UNFORMATTED);
}

#[test]
fn a_walk_follows_the_config_and_skips_excluded_directories() {
    let dir = repository(
        "walk_config",
        &[
            (
                ".scuff.toml",
                "include-paths = [\"glob:**.scala\", \"glob:**.txt\"]\nexclude-paths = [\"glob:**/gen/**\"]\n",
            ),
            ("A.scala", UNFORMATTED),
            ("notes.txt", UNFORMATTED),
            ("build.sbt", UNFORMATTED),
            ("gen/B.scala", UNFORMATTED),
        ],
    );
    let out = scuff_in(&dir, &["format", "--check", "."], "");
    assert_eq!(
        text(&out.stderr),
        "./A.scala: would be reformatted\n./notes.txt: would be reformatted\n"
    );
}

#[test]
fn a_file_named_twice_is_formatted_once() {
    let dir = repository("named_twice", &[("src/A.scala", UNFORMATTED)]);
    let out = scuff_in(
        &dir,
        &["format", "--check", "src", "src/A.scala", "./src"],
        "",
    );
    assert_eq!(text(&out.stderr), "src/A.scala: would be reformatted\n");
}

#[test]
fn a_broken_config_is_reported_once() {
    let dir = repository(
        "broken_walk",
        &[
            (".scalafmt.conf", "maxColumn = wide\n"),
            ("A.scala", UNFORMATTED),
            ("B.scala", UNFORMATTED),
        ],
    );
    let out = scuff_in(&dir, &["format", "."], "");
    assert_eq!(out.status.code(), Some(2));
    let stderr = text(&out.stderr);
    assert_eq!(stderr.lines().count(), 1, "{stderr}");
    assert!(stderr.contains(".scalafmt.conf: maxColumn"), "{stderr}");
    assert_eq!(read(&dir, "A.scala"), UNFORMATTED);
}

#[cfg(unix)]
#[test]
fn a_file_and_a_link_to_it_are_formatted_once() {
    let dir = repository("linked", &[("src/A.scala", UNFORMATTED)]);
    std::os::unix::fs::symlink("src/A.scala", dir.join("B.scala")).unwrap();
    let out = scuff_in(&dir, &["format", "--check", "src/A.scala", "B.scala"], "");
    assert_eq!(text(&out.stderr), "src/A.scala: would be reformatted\n");
    // A walk doesn't follow links.
    let out = scuff_in(&dir, &["format", "--check", "."], "");
    assert_eq!(text(&out.stderr), "./src/A.scala: would be reformatted\n");
}

#[test]
fn imports_are_rewritten_to_what_the_dialect_allows() {
    let dir = repository(
        "rewrite",
        &[
            (
                ".scalafmt.conf",
                "runner.dialect = scala3\nfileOverride { \".sbt\" = scala213 }\n",
            ),
            ("A.scala", "import a._\nimport b.{c => d}\n"),
            ("build.sbt", "import a._\n"),
        ],
    );
    let out = scuff_in(&dir, &["format", "."], "");
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(read(&dir, "A.scala"), "import a.*\nimport b.{ c as d }\n");
    assert_eq!(read(&dir, "build.sbt"), "import a._\n");
    // Standard input follows the dialect no pattern changes.
    let out = scuff_in(&dir, &["format"], "import a._\n");
    assert_eq!(text(&out.stdout), "import a.*\n");
}

#[test]
fn without_a_dialect_it_is_scala_3() {
    let dir = repository("no_dialect", &[("A.scala", "import a._\n")]);
    let out = scuff_in(&dir, &["format", "A.scala"], "");
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(read(&dir, "A.scala"), "import a.*\n");
}

#[test]
fn a_scala_2_dialect_keeps_imports() {
    let dir = repository(
        "scala_2",
        &[
            (".scuff.toml", "dialect = \"scala213\"\n"),
            ("A.scala", "import a._\n"),
        ],
    );
    let out = scuff_in(&dir, &["format", "A.scala"], "");
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(read(&dir, "A.scala"), "import a._\n");
}
