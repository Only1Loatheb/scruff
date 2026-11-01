//! Checks that rewritten code means the same as the original, for testing
//! formatters.
//!
//! Two texts are equivalent when:
//!
//! 1. The rewrite has no syntax errors unless the original did.
//! 2. Their trees have the same shape once whitespace and comments are set
//!    aside: the same nodes, nested the same way, with the same token text.
//!    Comparing the shape, not only the tokens, catches Scala 3 code moved
//!    into a different block by re-indenting.
//! 3. They have the same comments, in the same order, each attached to the
//!    same token in the same position (leading, trailing, or end of file).

use std::fmt;

use super::{
    NodeOrToken, Parse, SyntaxNode, SyntaxToken, SyntaxTrivia, TextSize, WalkEvent, parse,
};

/// The first way two texts differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Difference {
    pub kind: DifferenceKind,
    pub message: String,
    /// Where the difference is in the original, if it's there.
    pub before: Option<Location>,
    /// Where the difference is in the rewrite, if it's there.
    pub after: Option<Location>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferenceKind {
    /// The rewrite has a syntax error and the original doesn't.
    NewSyntaxError,
    /// The code differs: a token, or how nodes are nested.
    Code,
    /// A comment was dropped, added, changed, or attached somewhere else.
    Comment,
}

/// A position in one of the two texts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub offset: TextSize,
    /// 1-based.
    pub line: usize,
    /// 1-based, in characters.
    pub column: usize,
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;
        let at = |loc: Option<Location>| loc.map(|l| format!("{}:{}", l.line, l.column));
        match (at(self.before), at(self.after)) {
            (Some(b), Some(a)) => write!(f, " (before {b}, after {a})"),
            (Some(b), None) => write!(f, " (before {b})"),
            (None, Some(a)) => write!(f, " (after {a})"),
            (None, None) => Ok(()),
        }
    }
}

impl std::error::Error for Difference {}

/// Returns the first difference between `before` and `after`, or `Ok` if a
/// formatter could have produced `after` from `before`.
pub fn check_equivalent(before: &str, after: &str) -> Result<(), Difference> {
    let before = parse(before);
    let after = parse(after);
    check_errors(&before, &after)?;
    check_code(&before, &after)?;
    check_comments(&before, &after)
}

fn location(parse: &Parse, offset: TextSize) -> Location {
    let (line, column) = parse.line_col(offset);
    Location {
        offset,
        line,
        column,
    }
}

fn check_errors(before: &Parse, after: &Parse) -> Result<(), Difference> {
    match after.errors().first() {
        Some(error) if before.errors().is_empty() => Err(Difference {
            kind: DifferenceKind::NewSyntaxError,
            message: format!("the rewrite has a syntax error: {error}"),
            before: None,
            after: Some(location(after, error.range().start())),
        }),
        _ => Ok(()),
    }
}

// ---------------------------------------------------------------------------
// Code shape.

/// One step of a walk over the code, with trivia left out.
#[derive(Clone, Copy)]
enum Step<'t> {
    Enter(SyntaxNode<'t>),
    Leave(SyntaxNode<'t>),
    Token(SyntaxToken<'t>),
}

impl Step<'_> {
    fn same_as(&self, other: &Step<'_>) -> bool {
        match (self, other) {
            (Step::Enter(a), Step::Enter(b)) => {
                a.kind() == b.kind() && a.field_name() == b.field_name()
            }
            (Step::Leave(a), Step::Leave(b)) => a.kind() == b.kind(),
            (Step::Token(a), Step::Token(b)) => {
                a.kind() == b.kind()
                    && a.text() == b.text()
                    && a.is_missing() == b.is_missing()
                    && a.field_name() == b.field_name()
            }
            _ => false,
        }
    }

    fn offset(&self) -> TextSize {
        match self {
            Step::Enter(node) => node.text_range().start(),
            Step::Leave(node) => node.text_range().end(),
            Step::Token(token) => token.text_range().start(),
        }
    }

    fn describe(&self) -> String {
        let field = |name: Option<&str>| name.map_or(String::new(), |n| format!("{n}: "));
        match self {
            Step::Enter(node) => format!("{}{}", field(node.field_name()), node.kind()),
            Step::Leave(node) => format!("end of {}", node.kind()),
            Step::Token(token) if token.is_missing() => format!("missing {:?}", token.kind()),
            Step::Token(token) if token.kind().is_named() => {
                format!(
                    "{}{} {:?}",
                    field(token.field_name()),
                    token.kind(),
                    token.text()
                )
            }
            Step::Token(token) => format!("{}{:?}", field(token.field_name()), token.text()),
        }
    }
}

fn code_steps(root: SyntaxNode<'_>) -> impl Iterator<Item = Step<'_>> {
    root.preorder_with_tokens().filter_map(|event| match event {
        WalkEvent::Enter(element) | WalkEvent::Leave(element) if element.is_trivia() => None,
        WalkEvent::Enter(NodeOrToken::Node(node)) => Some(Step::Enter(node)),
        WalkEvent::Leave(NodeOrToken::Node(node)) => Some(Step::Leave(node)),
        WalkEvent::Enter(NodeOrToken::Token(token)) => Some(Step::Token(token)),
        WalkEvent::Leave(NodeOrToken::Token(_)) => None,
    })
}

fn check_code(before: &Parse, after: &Parse) -> Result<(), Difference> {
    let mut b = code_steps(before.syntax());
    let mut a = code_steps(after.syntax());
    loop {
        let (x, y) = (b.next(), a.next());
        let message = match (&x, &y) {
            (None, None) => return Ok(()),
            (Some(x), Some(y)) if x.same_as(y) => continue,
            (Some(x), Some(y)) => format!("expected {}, found {}", x.describe(), y.describe()),
            (Some(x), None) => format!("expected {}, found the end", x.describe()),
            (None, Some(y)) => format!("expected the end, found {}", y.describe()),
        };
        return Err(Difference {
            kind: DifferenceKind::Code,
            message,
            before: x.map(|s| location(before, s.offset())),
            after: y.map(|s| location(after, s.offset())),
        });
    }
}

// ---------------------------------------------------------------------------
// Comments.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Placement {
    Leading,
    Trailing,
    EndOfFile,
}

struct Comment<'t> {
    trivia: SyntaxTrivia<'t>,
    placement: Placement,
    /// Position of the owner among the code tokens, and its text. The code
    /// already matched, so the same index means the same token.
    owner: Option<(usize, &'t str)>,
}

impl Comment<'_> {
    fn describe_place(&self) -> String {
        match (self.placement, self.owner) {
            (Placement::Leading, Some((_, text))) => format!("before {text:?}"),
            (Placement::Trailing, Some((_, text))) => format!("after {text:?}"),
            _ => "at the end of the file".to_string(),
        }
    }
}

fn comments(parse: &Parse) -> Vec<Comment<'_>> {
    let mut out = Vec::new();
    let owners = parse.syntax().tokens().filter(SyntaxToken::owns_trivia);
    for (index, token) in owners.enumerate() {
        let owner = Some((index, token.text()));
        for (placement, trivia) in token
            .leading_trivia()
            .map(|t| (Placement::Leading, t))
            .chain(token.trailing_trivia().map(|t| (Placement::Trailing, t)))
        {
            if trivia.is_comment() {
                out.push(Comment {
                    trivia,
                    placement,
                    owner,
                });
            }
        }
    }
    for trivia in parse.eof_trivia().filter(SyntaxTrivia::is_comment) {
        out.push(Comment {
            trivia,
            placement: Placement::EndOfFile,
            owner: None,
        });
    }
    out
}

fn check_comments(before: &Parse, after: &Parse) -> Result<(), Difference> {
    let b = comments(before);
    let a = comments(after);
    for i in 0..b.len().max(a.len()) {
        let (x, y) = (b.get(i), a.get(i));
        let message = match (x, y) {
            (Some(x), Some(y)) if x.trivia.text() != y.trivia.text() => {
                format!("comment {:?} became {:?}", x.trivia.text(), y.trivia.text())
            }
            (Some(x), Some(y))
                if x.placement != y.placement || x.owner.map(|o| o.0) != y.owner.map(|o| o.0) =>
            {
                format!(
                    "comment {:?} moved from {} to {}",
                    x.trivia.text(),
                    x.describe_place(),
                    y.describe_place()
                )
            }
            (Some(_), Some(_)) => continue,
            (Some(x), None) => format!("comment {:?} is missing", x.trivia.text()),
            (None, Some(y)) => format!("comment {:?} was added", y.trivia.text()),
            (None, None) => unreachable!("i is below the longer length"),
        };
        return Err(Difference {
            kind: DifferenceKind::Comment,
            message,
            before: x.map(|c| location(before, c.trivia.text_range().start())),
            after: y.map(|c| location(after, c.trivia.text_range().start())),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn differs(before: &str, after: &str) -> Difference {
        check_equivalent(before, after).expect_err("should differ")
    }

    #[test]
    fn identical_text_is_equivalent() {
        let code = "object A {\n  def f(x: Int) = x + 1 // inc\n}\n";
        assert_eq!(check_equivalent(code, code), Ok(()));
    }

    #[test]
    fn whitespace_changes_in_brace_code_are_fine() {
        assert_eq!(
            check_equivalent(
                "object A{def f(x:Int)=x+1;val y=2}",
                "object A {\n  def f(x: Int) = x + 1; val y = 2\n}\n"
            ),
            Ok(())
        );
    }

    #[test]
    fn reindenting_own_line_comments_is_fine() {
        assert_eq!(
            check_equivalent(
                "object A {\n// note\n    val x = 1\n}",
                "object A {\n  // note\n  val x = 1\n}"
            ),
            Ok(())
        );
    }

    #[test]
    fn changed_token_is_reported_with_both_positions() {
        let d = differs("val x = 1\nval y = 2", "val x = 1\n\nval y = 3");
        assert_eq!(d.kind, DifferenceKind::Code);
        assert_eq!(
            d.message,
            r#"expected value: integer_literal "2", found value: integer_literal "3""#
        );
        assert_eq!((d.before.unwrap().line, d.before.unwrap().column), (2, 9));
        assert_eq!((d.after.unwrap().line, d.after.unwrap().column), (3, 9));
    }

    #[test]
    fn scala3_dedent_that_changes_blocks_is_caught() {
        // Same tokens, but `def g` leaves object A.
        let before = "object A:\n  def f = 1\n  def g = 2\n";
        let after = "object A:\n  def f = 1\ndef g = 2\n";
        assert_eq!(differs(before, after).kind, DifferenceKind::Code);
        // Shifting the whole object keeps its structure.
        let shifted = "object A:\n    def f = 1\n    def g = 2\n";
        assert_eq!(check_equivalent(before, shifted), Ok(()));
    }

    #[test]
    fn whitespace_inside_strings_matters() {
        assert_eq!(
            differs(r#"val s = "a b""#, r#"val s = "a  b""#).kind,
            DifferenceKind::Code
        );
        assert_eq!(
            differs(r#"val s = s"a ${x} b""#, r#"val s = s"a ${x}  b""#).kind,
            DifferenceKind::Code
        );
        let multiline = "val s = \"\"\"a\n  b\"\"\"";
        assert_eq!(
            differs(multiline, "val s = \"\"\"a\n b\"\"\"").kind,
            DifferenceKind::Code
        );
    }

    #[test]
    fn new_syntax_error_is_reported() {
        let d = differs("val x = (1 + 2)", "val x = (1 + 2");
        assert_eq!(d.kind, DifferenceKind::NewSyntaxError);
        assert!(d.before.is_none() && d.after.is_some(), "{d}");
    }

    #[test]
    fn untouched_broken_code_is_equivalent() {
        let broken = "object A {\n  def f( = 1\n";
        assert_eq!(check_equivalent(broken, broken), Ok(()));
    }

    #[test]
    fn dropped_comment_is_reported() {
        let d = differs("val x = 1 // keep me\n", "val x = 1\n");
        assert_eq!(d.kind, DifferenceKind::Comment);
        assert_eq!(d.message, r#"comment "// keep me" is missing"#);
    }

    #[test]
    fn added_comment_is_reported() {
        let d = differs("val x = 1\n", "val x = 1 /* new */\n");
        assert_eq!(d.message, r#"comment "/* new */" was added"#);
        assert!(d.before.is_none());
    }

    #[test]
    fn edited_comment_is_reported() {
        let d = differs("val x = 1 // teh\n", "val x = 1 // the\n");
        assert_eq!(d.message, r#"comment "// teh" became "// the""#);
    }

    #[test]
    fn comment_moved_to_another_token_is_reported() {
        let d = differs("f(a, // first\n  b)", "f(a,\n  // first\n  b)");
        assert_eq!(d.kind, DifferenceKind::Comment);
        assert_eq!(
            d.message,
            r#"comment "// first" moved from after "," to before "b""#
        );
    }

    #[test]
    fn comment_moved_to_end_of_file_is_reported() {
        let d = differs("val x = 1 // c\n", "val x = 1\n// c\n");
        assert_eq!(
            d.message,
            r#"comment "// c" moved from after "1" to at the end of the file"#
        );
    }

    #[test]
    fn display_includes_positions() {
        let d = differs("val x = 1", "val x = 2");
        assert_eq!(
            d.to_string(),
            r#"expected value: integer_literal "1", found value: integer_literal "2" (before 1:9, after 1:9)"#
        );
    }
}
