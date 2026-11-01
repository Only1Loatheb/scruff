//! A Wadler-style pretty-printing document, in the spirit of Prettier's
//! `Doc`.
//!
//! A formatter describes output as a [`Doc`]: text, places where a line may
//! break, groups that should stay on one line if they fit, and indentation.
//! [`print`] then picks the line breaks for a given width. Each group is
//! printed flat (every `line` becomes a space, every `softline` nothing) if
//! the group and the rest of its line fit, and broken (every `line` and
//! `softline` becomes a newline) otherwise. Groups decide outermost first,
//! so an outer group can break while the groups inside it stay flat.
//!
//! ```
//! use ouf::doc::{Doc, PrintOptions, print};
//!
//! let args = Doc::join(["first", "second", "third"].map(Doc::text), Doc::text(",") + Doc::line());
//! let call = Doc::text("f(") + (Doc::softline() + args).nest() + Doc::softline() + Doc::text(")");
//! let doc = call.group();
//! assert_eq!(print(&doc, &PrintOptions::default()), "f(first, second, third)");
//! let narrow = PrintOptions { max_width: 10, ..PrintOptions::default() };
//! assert_eq!(print(&doc, &narrow), "f(\n  first,\n  second,\n  third\n)");
//! ```

use std::borrow::Cow;
use std::ops::Add;

/// A document to print. Text can borrow from the source being formatted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Doc<'a> {
    Nil,
    /// Text without line breaks.
    Text(Cow<'a, str>),
    /// Text printed exactly as given. It may contain line breaks; lines
    /// after the first are not indented. A line break forces the enclosing
    /// groups to break.
    Verbatim(Cow<'a, str>),
    Line(LineKind),
    Concat(Vec<Doc<'a>>),
    /// Indents the lines that start inside it by one level.
    Nest(Box<Doc<'a>>),
    /// Printed flat if it fits, broken otherwise.
    Group(Box<Doc<'a>>),
    /// Prints `broken` if the enclosing group breaks, `flat` if not.
    IfBreak {
        broken: Box<Doc<'a>>,
        flat: Box<Doc<'a>>,
    },
    /// Printed at the end of the current line, just before the next line
    /// break, such as a trailing `// comment`. Takes no width.
    LineSuffix(Box<Doc<'a>>),
    /// Forces the enclosing groups to break. Use with a line suffix that
    /// must end its line, such as a `//` comment.
    BreakParent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    /// A space when flat, a line break when broken.
    Line,
    /// Nothing when flat, a line break when broken.
    Soft,
    /// Always a line break; forces the enclosing groups to break.
    Hard,
}

impl<'a> Doc<'a> {
    /// Text without line breaks. Use [`Doc::verbatim`] for text that has them.
    pub fn text(text: impl Into<Cow<'a, str>>) -> Doc<'a> {
        let text = text.into();
        debug_assert!(
            !text.contains(['\n', '\r']),
            "Doc::text with a line break: {text:?}"
        );
        Doc::Text(text)
    }

    pub fn verbatim(text: impl Into<Cow<'a, str>>) -> Doc<'a> {
        Doc::Verbatim(text.into())
    }

    pub fn line() -> Doc<'a> {
        Doc::Line(LineKind::Line)
    }

    pub fn softline() -> Doc<'a> {
        Doc::Line(LineKind::Soft)
    }

    pub fn hardline() -> Doc<'a> {
        Doc::Line(LineKind::Hard)
    }

    pub fn concat(docs: impl IntoIterator<Item = Doc<'a>>) -> Doc<'a> {
        Doc::Concat(docs.into_iter().collect())
    }

    /// The docs with `separator` between each pair.
    pub fn join(docs: impl IntoIterator<Item = Doc<'a>>, separator: Doc<'a>) -> Doc<'a> {
        let mut parts = Vec::new();
        for (i, doc) in docs.into_iter().enumerate() {
            if i > 0 {
                parts.push(separator.clone());
            }
            parts.push(doc);
        }
        Doc::Concat(parts)
    }

    pub fn nest(self) -> Doc<'a> {
        Doc::Nest(Box::new(self))
    }

    pub fn group(self) -> Doc<'a> {
        Doc::Group(Box::new(self))
    }

    pub fn if_break(broken: Doc<'a>, flat: Doc<'a>) -> Doc<'a> {
        Doc::IfBreak {
            broken: Box::new(broken),
            flat: Box::new(flat),
        }
    }

    pub fn line_suffix(self) -> Doc<'a> {
        Doc::LineSuffix(Box::new(self))
    }

    pub fn break_parent() -> Doc<'a> {
        Doc::BreakParent
    }
}

/// `a + b` concatenates, flattening nested concatenations.
impl<'a> Add for Doc<'a> {
    type Output = Doc<'a>;

    fn add(self, rhs: Doc<'a>) -> Doc<'a> {
        let mut parts = match self {
            Doc::Concat(parts) => parts,
            Doc::Nil => Vec::new(),
            other => vec![other],
        };
        match rhs {
            Doc::Concat(more) => parts.extend(more),
            Doc::Nil => {}
            other => parts.push(other),
        }
        Doc::Concat(parts)
    }
}

impl<'a> From<&'a str> for Doc<'a> {
    fn from(text: &'a str) -> Doc<'a> {
        Doc::text(text)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrintOptions {
    /// Lines longer than this, in characters, make groups break. Defaults
    /// to 100, sbt's scalafmt `maxColumn`.
    pub max_width: usize,
    /// One level of indentation.
    pub indent: String,
    /// What a line break prints as, such as `"\n"` or `"\r\n"`.
    pub line_ending: String,
}

impl Default for PrintOptions {
    fn default() -> Self {
        PrintOptions {
            max_width: 100,
            indent: "  ".to_string(),
            line_ending: "\n".to_string(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Flat,
    Break,
}

#[derive(Clone, Copy)]
struct Command<'d, 'a> {
    indent: usize,
    mode: Mode,
    doc: &'d Doc<'a>,
}

fn width(text: &str) -> usize {
    text.chars().count()
}

/// Lays out `doc` within `options.max_width` where it can. Lines never end
/// in spaces or tabs, unless verbatim text put them there.
pub fn print(doc: &Doc<'_>, options: &PrintOptions) -> String {
    let indent_width = width(&options.indent);
    let mut out = String::new();
    let mut column = 0;
    let mut commands = vec![Command {
        indent: 0,
        mode: Mode::Break,
        doc,
    }];
    let mut suffixes: Vec<Command<'_, '_>> = Vec::new();
    // Trimming trailing spaces must not reach back into verbatim text.
    let mut verbatim_end = 0;

    while let Some(command) = commands.pop() {
        let Command { indent, mode, doc } = command;
        match doc {
            Doc::Nil | Doc::BreakParent => {}
            Doc::Text(text) => {
                out.push_str(text);
                column += width(text);
            }
            Doc::Verbatim(text) => {
                out.push_str(text);
                verbatim_end = out.len();
                column = match text.rfind(['\n', '\r']) {
                    Some(i) => width(&text[i + 1..]),
                    None => column + width(text),
                };
            }
            Doc::Concat(parts) => {
                commands.extend(parts.iter().rev().map(|doc| Command { indent, mode, doc }));
            }
            Doc::Nest(inner) => commands.push(Command {
                indent: indent + 1,
                mode,
                doc: inner,
            }),
            Doc::Group(inner) => {
                let flat = Command {
                    indent,
                    mode: Mode::Flat,
                    doc: inner,
                };
                let room = options.max_width as isize - column as isize;
                let mode = if mode == Mode::Flat || fits(flat, &commands, room) {
                    Mode::Flat
                } else {
                    Mode::Break
                };
                commands.push(Command {
                    indent,
                    mode,
                    doc: inner,
                });
            }
            Doc::IfBreak { broken, flat } => commands.push(Command {
                indent,
                mode,
                doc: if mode == Mode::Break { broken } else { flat },
            }),
            Doc::LineSuffix(inner) => suffixes.push(Command {
                indent,
                mode,
                doc: inner,
            }),
            Doc::Line(kind) => match (mode, kind) {
                (Mode::Flat, LineKind::Soft) => {}
                (Mode::Flat, LineKind::Line) => {
                    out.push(' ');
                    column += 1;
                }
                _ => {
                    if !suffixes.is_empty() {
                        // Print pending suffixes first, then come back here.
                        commands.push(command);
                        commands.extend(suffixes.drain(..).rev());
                        continue;
                    }
                    let trimmed = out.trim_end_matches([' ', '\t']).len();
                    out.truncate(trimmed.max(verbatim_end));
                    out.push_str(&options.line_ending);
                    for _ in 0..indent {
                        out.push_str(&options.indent);
                    }
                    column = indent * indent_width;
                }
            },
        }
        if commands.is_empty() && !suffixes.is_empty() {
            commands.extend(suffixes.drain(..).rev());
        }
    }
    let trimmed = out.trim_end_matches([' ', '\t']).len();
    out.truncate(trimmed.max(verbatim_end));
    out
}

/// Whether `next`, printed in its mode, and then the rest of the current
/// line fit in `room` columns.
fn fits(next: Command<'_, '_>, rest: &[Command<'_, '_>], mut room: isize) -> bool {
    let mut stack = vec![(next.mode, next.doc)];
    let mut rest = rest.iter().rev();
    loop {
        if room < 0 {
            return false;
        }
        let (mode, doc) = match stack.pop() {
            Some(item) => item,
            None => match rest.next() {
                Some(command) => (command.mode, command.doc),
                None => return true,
            },
        };
        match doc {
            Doc::Nil | Doc::LineSuffix(_) => {}
            Doc::Text(text) => room -= width(text) as isize,
            Doc::Verbatim(text) => match text.find(['\n', '\r']) {
                Some(_) if mode == Mode::Flat => return false,
                Some(i) => return room >= width(&text[..i]) as isize,
                None => room -= width(text) as isize,
            },
            Doc::Line(LineKind::Hard) | Doc::BreakParent if mode == Mode::Flat => return false,
            Doc::BreakParent => {}
            Doc::Line(_) if mode == Mode::Break => return true,
            Doc::Line(LineKind::Line) => room -= 1,
            Doc::Line(_) => {}
            Doc::Concat(parts) => stack.extend(parts.iter().rev().map(|doc| (mode, doc))),
            Doc::Nest(inner) | Doc::Group(inner) => stack.push((mode, inner)),
            Doc::IfBreak { broken, flat } => {
                stack.push((mode, if mode == Mode::Break { broken } else { flat }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(width: usize, doc: &Doc<'_>) -> String {
        print(
            doc,
            &PrintOptions {
                max_width: width,
                ..PrintOptions::default()
            },
        )
    }

    fn t(text: &str) -> Doc<'_> {
        Doc::text(text)
    }

    /// `name(arg, arg, ...)`, breaking one argument per line.
    fn call<'a>(name: &'a str, args: Vec<Doc<'a>>) -> Doc<'a> {
        let args = Doc::join(args, t(",") + Doc::line());
        (t(name) + t("(") + (Doc::softline() + args).nest() + Doc::softline() + t(")")).group()
    }

    #[test]
    fn group_stays_flat_when_it_fits() {
        let doc = call("f", vec![t("a"), t("b")]);
        assert_eq!(at(80, &doc), "f(a, b)");
        assert_eq!(at(7, &doc), "f(a, b)", "exactly the width fits");
        assert_eq!(at(6, &doc), "f(\n  a,\n  b\n)");
    }

    #[test]
    fn outer_group_breaks_first() {
        let doc = call(
            "outer",
            vec![call("inner", vec![t("x"), t("y")]), t("zzzzzzzzzz")],
        );
        assert_eq!(at(80, &doc), "outer(inner(x, y), zzzzzzzzzz)");
        assert_eq!(at(20, &doc), "outer(\n  inner(x, y),\n  zzzzzzzzzz\n)");
        assert_eq!(
            at(10, &doc),
            "outer(\n  inner(\n    x,\n    y\n  ),\n  zzzzzzzzzz\n)"
        );
    }

    #[test]
    fn the_rest_of_the_line_counts() {
        // The group fits alone, but not with the text after it.
        let doc = call("f", vec![t("a")]) + t(".method()");
        assert_eq!(at(12, &doc), "f(\n  a\n).method()");
        assert_eq!(at(13, &doc), "f(a).method()");
    }

    #[test]
    fn hardline_forces_its_groups_to_break() {
        let doc = (t("{")
            + (Doc::line() + t("a") + Doc::hardline() + t("b")).nest()
            + Doc::line()
            + t("}"))
        .group();
        assert_eq!(at(80, &doc), "{\n  a\n  b\n}");
    }

    #[test]
    fn if_break_adds_a_trailing_comma_only_when_broken() {
        let args =
            Doc::join(vec![t("a"), t("b")], t(",") + Doc::line()) + Doc::if_break(t(","), Doc::Nil);
        let doc = (t("f(") + (Doc::softline() + args).nest() + Doc::softline() + t(")")).group();
        assert_eq!(at(80, &doc), "f(a, b)");
        assert_eq!(at(4, &doc), "f(\n  a,\n  b,\n)");
    }

    #[test]
    fn line_suffix_goes_to_the_end_of_the_line() {
        let comment = t(" // note").line_suffix();
        let doc = t("val a = 1") + comment + t(";") + Doc::hardline() + t("val b = 2");
        assert_eq!(at(80, &doc), "val a = 1; // note\nval b = 2");
        // At the very end, the suffix still gets printed.
        assert_eq!(at(80, &(t("x") + t(" // end").line_suffix())), "x // end");
    }

    #[test]
    fn line_comment_with_break_parent_keeps_its_line() {
        // `f(a, // first` / `b)`: the comment must stay after `a,`.
        let comment = t(" // first").line_suffix() + Doc::break_parent();
        let args = t("a") + t(",") + comment + Doc::line() + t("b");
        let doc = (t("f(") + (Doc::softline() + args).nest() + Doc::softline() + t(")")).group();
        assert_eq!(at(80, &doc), "f(\n  a, // first\n  b\n)");
    }

    #[test]
    fn verbatim_keeps_its_lines_and_breaks_groups() {
        let string = Doc::verbatim("\"\"\"one\n  two\"\"\"");
        let doc = (t("f(") + (Doc::softline() + string).nest() + Doc::softline() + t(")")).group();
        assert_eq!(at(80, &doc), "f(\n  \"\"\"one\n  two\"\"\"\n)");
        // The column continues from the verbatim text's last line.
        let after = Doc::verbatim("ab\ncd") + call("g", vec![t("x")]);
        assert_eq!(at(6, &after), "ab\ncdg(x)");
        assert_eq!(at(5, &after), "ab\ncdg(\n  x\n)");
    }

    #[test]
    fn verbatim_keeps_trailing_spaces_but_other_lines_lose_them() {
        let doc = t("a ")
            + Doc::hardline()
            + Doc::hardline()
            + Doc::verbatim("b  \nc  ")
            + Doc::hardline();
        assert_eq!(at(80, &doc.nest()), "a\n\n  b  \nc  \n");
    }

    #[test]
    fn nest_indents_lines_started_inside_it() {
        let body = (Doc::hardline() + t("val x = 1") + Doc::hardline() + t("x")).nest();
        let doc = t("def f =") + body + Doc::hardline() + t("end f");
        assert_eq!(at(80, &doc), "def f =\n  val x = 1\n  x\nend f");
        let tabs = PrintOptions {
            indent: "\t".to_string(),
            ..PrintOptions::default()
        };
        assert_eq!(print(&doc, &tabs), "def f =\n\tval x = 1\n\tx\nend f");
    }

    #[test]
    fn line_ending_is_configurable() {
        let crlf = PrintOptions {
            line_ending: "\r\n".to_string(),
            ..PrintOptions::default()
        };
        let doc = t("a") + Doc::hardline() + t("b");
        assert_eq!(print(&doc, &crlf), "a\r\nb");
    }

    #[test]
    fn width_counts_characters() {
        let doc = call("f", vec![t("日本語"), t("é")]);
        assert_eq!(at(9, &doc), "f(日本語, é)");
        assert_eq!(at(8, &doc), "f(\n  日本語,\n  é\n)");
    }

    #[test]
    fn add_flattens() {
        let doc = t("a") + t("b") + Doc::Nil + (t("c") + t("d"));
        assert_eq!(doc, Doc::Concat(vec![t("a"), t("b"), t("c"), t("d")]));
    }

    #[test]
    fn deep_nesting_does_not_overflow() {
        let mut doc = t("x");
        for _ in 0..1000 {
            doc = call("f", vec![doc]);
        }
        let out = at(80, &doc);
        assert_eq!(out.matches('\n').count(), out.lines().count() - 1);
        assert!(out.starts_with("f(\n"));
    }
}
