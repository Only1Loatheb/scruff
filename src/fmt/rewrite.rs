//! Rewrites to the syntax of a file's dialect, made before formatting:
//! `import a._` to `import a.*`, and `import a.{b => c}` to
//! `import a.{b as c}`, in imports and exports alike, where the dialect
//! allows the new syntax, and back where it doesn't. A rename without
//! braces, `import a.b as c`, gets them: `import a.{b => c}`.
//!
//! A type wildcard `_` becomes `?`, as in `List[?]`, where the dialect
//! allows it. Not back: without it, `?` is a type's name.
//!
//! A vararg splice `f(xs: _*)` becomes `f(xs*)`, and `case Seq(xs @ _*)`
//! becomes `case Seq(xs*)`, and back.
//!
//! Scala 3's control syntax: `if (c) a` becomes `if c then a`,
//! `while (c) x` becomes `while c do x`, and `for (x <- xs) f(x)` becomes
//! `for x <- xs do f(x)`, and back. `for` braces stay; back in Scala 2, a
//! `for` written over several lines gets them.
//!
//! Only tokens change; comments and whitespace stay where they are, and
//! code copied verbatim, such as `format: off` blocks, is left alone.
//! `check_equivalent` treats each old form and its new one as the same.

use crate::config::Dialect;
use crate::lossless_tree::{
    NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, Tree, is_type_wildcard_place,
};

/// The source rewritten for `dialect`, or `None` if nothing changes.
pub(super) fn rewrite(tree: &Tree, dialect: Dialect) -> Option<String> {
    let text = tree.syntax().text();
    let mut edits: Vec<(TextRange, String)> = Vec::new();
    for node in tree.syntax().descendants() {
        let tokens = || {
            node.children_with_tokens()
                .filter_map(NodeOrToken::into_token)
                .filter(|t| !t.is_trivia() && tree.verbatim_at(t.text_range().start()).is_none())
        };
        match node.kind() {
            SyntaxKind::NamespaceWildcard if dialect.star_wildcard_import => {
                for token in tokens().filter(|t| t.text() == "_") {
                    edits.push((token.text_range(), "*".to_string()));
                }
            }
            SyntaxKind::NamespaceWildcard => {
                for token in tokens().filter(|t| t.text() == "*") {
                    edits.push((token.text_range(), "_".to_string()));
                }
            }
            SyntaxKind::Wildcard
                if dialect.question_mark_type_wildcard
                    && is_type_wildcard_place(NodeOrToken::Node(node)) =>
            {
                for token in tokens().filter(|t| t.text() == "_") {
                    // `_<:A` is fine, but `?<:` would be one operator.
                    let end = usize::from(token.text_range().end());
                    let joins = text[end..].chars().next().is_some_and(is_operator_char);
                    let mark = if joins { "? " } else { "?" };
                    edits.push((token.text_range(), mark.to_string()));
                }
            }
            SyntaxKind::Vararg | SyntaxKind::RepeatPattern => {
                if let Some(edit) = splice(text, node, dialect.postfix_star_vararg_splices)
                    .filter(|(range, _)| tree.verbatim_at(range.start()).is_none())
                {
                    edits.push(edit);
                }
            }
            SyntaxKind::IfExpression | SyntaxKind::WhileExpression | SyntaxKind::ForExpression => {
                let control = control(text, node, dialect.quiet_syntax).filter(|edits| {
                    edits
                        .iter()
                        .all(|(range, _)| tree.verbatim_at(range.start()).is_none())
                });
                edits.extend(control.into_iter().flatten());
            }
            SyntaxKind::ArrowRenamedIdentifier if dialect.as_for_import_rename => {
                for token in tokens().filter(|t| matches!(t.text(), "=>" | "\u{21D2}")) {
                    edits.push((token.text_range(), spaced(text, &token, "as")));
                }
            }
            SyntaxKind::AsRenamedIdentifier if !dialect.as_for_import_rename => {
                let arrows: Vec<_> = tokens().filter(|t| t.text() == "as").collect();
                // `import foo as f` has no Scala 2 form: braces need a path
                // before them.
                if arrows.is_empty() || !after_a_path(node) {
                    continue;
                }
                for token in arrows {
                    edits.push((token.text_range(), spaced(text, &token, "=>")));
                }
                // Scala 2 renames only in braces.
                let braced = node
                    .parent()
                    .is_some_and(|p| p.kind() == SyntaxKind::NamespaceSelectors);
                if let (false, Some(first), Some(last)) =
                    (braced, node.first_token(), node.last_code_token())
                {
                    let start = first.text_range().start();
                    let end = last.text_range().end();
                    edits.push((TextRange::new(start, start), "{".to_string()));
                    edits.push((TextRange::new(end, end), "}".to_string()));
                }
            }
            _ => {}
        }
    }
    if edits.is_empty() {
        return None;
    }
    edits.sort_by_key(|(range, _)| range.start());
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    for (range, replacement) in edits {
        out.push_str(&text[copied..usize::from(range.start())]);
        out.push_str(&replacement);
        copied = usize::from(range.end());
    }
    out.push_str(&text[copied..]);
    Some(out)
}

/// Whether a rename comes after a path and its `.`, as `c` in
/// `import a.b.c as d`, or in braces after one.
pub(crate) fn after_a_path(rename: SyntaxNode<'_>) -> bool {
    if rename
        .parent()
        .is_some_and(|p| p.kind() == SyntaxKind::NamespaceSelectors)
    {
        return true;
    }
    rename
        .first_token()
        .and_then(|t| {
            std::iter::successors(t.prev_token(), SyntaxToken::prev_token).find(|t| !t.is_trivia())
        })
        .is_some_and(|t| t.text() == ".")
}

/// The edit that writes a vararg splice, `node`, as `xs*` when `postfix`,
/// or else as `xs: _*`, or in a pattern `xs @ _*`: the text between `xs`
/// and `*` replaced. None if it's already written so, or if a comment is in
/// the way. `_*` in a pattern is the same in both.
pub(super) fn splice(
    text: &str,
    node: SyntaxNode<'_>,
    postfix: bool,
) -> Option<(TextRange, String)> {
    let mut children = node.non_trivia_children();
    let (first, star) = (children.next()?, node.last_child_or_token()?);
    if star.text() != "*" {
        return None;
    }
    // The spliced name or expression.
    let (spliced, written) = match (node.kind(), first.kind()) {
        (SyntaxKind::Vararg, _) => {
            let colon = children.next().is_some_and(|c| c.text() == ":");
            (first, colon)
        }
        (_, SyntaxKind::CapturePattern) => {
            let first = first.into_node()?;
            if first.child_by_field_name("pattern")?.kind() != SyntaxKind::Wildcard {
                return None;
            }
            (first.child_by_field_name("name")?, true)
        }
        (_, SyntaxKind::Identifier) => (first, false),
        _ => return None,
    };
    if postfix != written {
        return None;
    }
    let range = TextRange::new(spliced.text_range().end(), star.text_range().start());
    let between = &text[range];
    let in_the_way = if postfix {
        // Only `: _` or `@ _` and spaces; and `xs*` mustn't make one name,
        // as `foo_*` or `++*` would.
        let last = spliced.text().chars().next_back()?;
        !between
            .chars()
            .all(|c| c.is_whitespace() || matches!(c, ':' | '@' | '_'))
            || last == '_'
            || is_operator_char(last)
    } else {
        !between.chars().all(char::is_whitespace)
    };
    if in_the_way {
        return None;
    }
    let replacement = match (postfix, node.kind()) {
        (true, _) => "",
        (false, SyntaxKind::Vararg) => ": _",
        (false, _) => " @ _",
    };
    Some((range, replacement.to_string()))
}

/// The edits that write a control expression, `node`, in Scala 3's syntax
/// when `quiet`, or else in Scala 2's. None if it's already written so, or
/// if it's left as written: where a line break or comment is in the way,
/// or where a condition starts with `(`, so taking the parentheses away
/// could leave it looking like the old syntax.
pub(super) fn control(
    text: &str,
    node: SyntaxNode<'_>,
    quiet: bool,
) -> Option<Vec<(TextRange, String)>> {
    let child = |text_of: &str, field: Option<&str>| {
        node.children_with_tokens()
            .filter_map(NodeOrToken::into_token)
            .find(|t| t.text() == text_of && (field.is_none() || t.field_name() == field))
    };
    if misread_either_way(text, node, quiet) {
        return None;
    }
    let between =
        |start: TextRange, end: TextRange| &text[TextRange::new(start.end(), end.start())];
    let blank = |s: &str| s.chars().all(char::is_whitespace);
    let one_line = |s: &str| !s.contains('\n');
    let mut edits = Vec::new();
    if node.kind() == SyntaxKind::ForExpression {
        let open = child("(", Some("enumerators")).or_else(|| child("{", Some("enumerators")));
        let close = child(")", Some("enumerators")).or_else(|| child("}", Some("enumerators")));
        let (done, yields) = (child("do", None), child("yield", None));
        // The node, not the brackets around it, which share its field.
        let enums = node
            .children()
            .find(|c| c.kind() == SyntaxKind::Enumerators)
            .map(NodeOrToken::Node);
        match (quiet, open, close) {
            // `for (x <- xs) f(x)` to `for x <- xs do f(x)`; braces stay.
            (true, Some(open), Some(close)) if open.text() == "(" => {
                let enums = code_range(&enums?)?;
                if !one_line(between(open.text_range(), enums))
                    || !one_line(between(enums, close.text_range()))
                {
                    return None;
                }
                edits.push((open.text_range(), unspaced(text, &open)));
                let after = if done.is_some() || yields.is_some() {
                    unspaced(text, &close)
                } else {
                    spaced(text, &close, "do")
                };
                edits.push((close.text_range(), after));
            }
            // A `do` after parentheses or braces goes.
            (false, Some(_), Some(close)) => {
                let done = done?;
                let gap = TextRange::new(close.text_range().end(), done.text_range().end());
                if !blank(&text[TextRange::new(gap.start(), done.text_range().start())]) {
                    return None;
                }
                edits.push((gap, String::new()));
            }
            (false, None, None) => {
                let keyword = done.or(yields)?;
                let first = node.first_token()?;
                let enums = code_range(&enums?)?;
                let gap = between(enums, keyword.text_range());
                if !blank(gap) {
                    return None;
                }
                let several_lines =
                    !one_line(between(first.text_range(), enums)) || !one_line(&text[enums]);
                let done = keyword.text() == "do";
                if !several_lines {
                    // `for x <- xs do f(x)` to `for (x <- xs) f(x)`.
                    edits.push((TextRange::new(enums.start(), enums.start()), "(".into()));
                    if done {
                        let range = TextRange::new(enums.end(), keyword.text_range().end());
                        edits.push((range, ")".into()));
                    } else {
                        edits.push((TextRange::new(enums.end(), enums.end()), ")".into()));
                    }
                } else {
                    // In braces, which allow line breaks between
                    // enumerators.
                    let end = first.text_range().end();
                    edits.push((TextRange::new(end, end), " {".into()));
                    let (range, close) = match (one_line(gap), done) {
                        // `}` on the keyword's line.
                        (false, true) => (keyword.text_range(), "}"),
                        (false, false) => (keyword.text_range(), "} yield"),
                        (true, true) => (
                            TextRange::new(enums.end(), keyword.text_range().end()),
                            " }",
                        ),
                        (true, false) => (TextRange::new(enums.end(), enums.end()), " }"),
                    };
                    edits.push((range, close.into()));
                }
            }
            _ => return None,
        }
        return Some(edits);
    }
    let keyword_text = if node.kind() == SyntaxKind::IfExpression {
        "then"
    } else {
        "do"
    };
    let keyword = child(keyword_text, Some("condition"));
    let condition = node.child_by_field_name("condition")?;
    let parens = condition.kind() == SyntaxKind::ParenthesizedExpression;
    match (quiet, parens, keyword) {
        // `if (c) a` to `if c then a`.
        (true, true, keyword) => {
            let condition = condition.into_node()?;
            let mut children = condition.non_trivia_children();
            let (open, inner, close) = (children.next()?, children.next()?, children.next()?);
            if children.next().is_some() || open.text() != "(" || close.text() != ")" {
                return None;
            }
            let (open, close) = (open.into_token()?, close.into_token()?);
            let inner = code_range(&inner)?;
            if !one_line(between(open.text_range(), inner))
                || !one_line(between(inner, close.text_range()))
                || text[inner].starts_with('(')
            {
                return None;
            }
            edits.push((open.text_range(), unspaced(text, &open)));
            let after = if keyword.is_some() {
                unspaced(text, &close)
            } else {
                spaced(text, &close, keyword_text)
            };
            edits.push((close.text_range(), after));
        }
        // `if (c) then a` to `if (c) a`.
        (false, true, Some(keyword)) => {
            let condition = code_range(&condition)?;
            if !blank(between(condition, keyword.text_range())) {
                return None;
            }
            let range = TextRange::new(condition.end(), keyword.text_range().end());
            edits.push((range, String::new()));
        }
        // `if c then a` to `if (c) a`.
        (false, false, Some(keyword)) => {
            let first = node.first_token()?;
            let condition = code_range(&condition)?;
            if !one_line(between(first.text_range(), condition))
                || !blank(between(condition, keyword.text_range()))
                || !one_line(between(condition, keyword.text_range()))
            {
                return None;
            }
            let start = condition.start();
            edits.push((TextRange::new(start, start), "(".into()));
            let range = TextRange::new(condition.end(), keyword.text_range().end());
            edits.push((range, ")".into()));
        }
        _ => return None,
    }
    Some(edits)
}

/// Whether tree-sitter-scala reads `node` differently in one of the two
/// syntaxes, so rewriting it would change the tree:
///
/// - A body that is a `match`: `for (x <- xs) x match { ... }` is read
///   with the `for` as what's matched, unlike `for x <- xs do x match`.
/// - An `if` ending another's consequence, where only one has an `else`:
///   which one it goes to can change, as in `if a then x = if b then 1
///   else 2`, where it goes to the first.
/// - An end marker: `end if` is one only after `if c then`, and a call
///   after `if (c)`.
/// - `inline if`, which Scala 2 doesn't have.
fn misread_either_way(text: &str, node: SyntaxNode<'_>, quiet: bool) -> bool {
    let body = node
        .child_by_field_name("body")
        .or_else(|| node.child_by_field_name("consequence"));
    let matched = node.field_name() == Some("value")
        && node
            .parent()
            .is_some_and(|p| p.kind() == SyntaxKind::MatchExpression);
    if matched || body.is_some_and(|b| b.kind() == SyntaxKind::MatchExpression) {
        return true;
    }
    if node.kind() == SyntaxKind::IfExpression && dangling_else(node) {
        return true;
    }
    let mut tokens = node.tokens().filter(|t| !t.is_trivia());
    let mut keyword = tokens.next().map_or("", |t| t.text());
    let inline = keyword == "inline";
    if inline {
        keyword = tokens.next().map_or("", |t| t.text());
    }
    let after = text[usize::from(node.text_range().end())..].trim_start();
    let end_after = after
        .strip_prefix("end")
        .is_some_and(|rest| rest.trim_start_matches([' ', '\t']).starts_with(keyword));
    let end_inside = node.children().any(|c| c.kind() == SyntaxKind::EndMarker);
    end_after || end_inside || (!quiet && inline)
}

/// Whether `node`, an `if`, ends another's consequence or another ends
/// its own, where only one of the two has an `else`.
fn dangling_else(node: SyntaxNode<'_>) -> bool {
    let has_else = |n: &SyntaxNode<'_>| {
        n.children_with_tokens()
            .any(|c| c.as_token().is_some_and(|t| t.text() == "else"))
    };
    let end = |n: &NodeOrToken<SyntaxNode<'_>, SyntaxToken<'_>>| code_range(n).map(|r| r.end());
    // An `if` ending this one's consequence.
    if let Some(NodeOrToken::Node(c)) = node.child_by_field_name("consequence") {
        let c_end = end(&NodeOrToken::Node(c));
        if c.descendants().any(|d| {
            d.kind() == SyntaxKind::IfExpression
                && end(&NodeOrToken::Node(d)) == c_end
                && has_else(&d) != has_else(&node)
        }) {
            return true;
        }
    }
    // An `if` whose consequence this one ends.
    let node_end = end(&NodeOrToken::Node(node));
    std::iter::successors(node.parent(), SyntaxNode::parent).any(|a| {
        a.kind() == SyntaxKind::IfExpression
            && has_else(&a) != has_else(&node)
            && a.child_by_field_name("consequence").is_some_and(|c| {
                c.text_range().contains_range(node.text_range()) && end(&c) == node_end
            })
    })
}

/// Where an element's code starts and ends, without the trivia and line
/// breaks at its edges.
fn code_range(element: &NodeOrToken<SyntaxNode<'_>, SyntaxToken<'_>>) -> Option<TextRange> {
    let node = match element {
        NodeOrToken::Token(token) => return Some(token.text_range()),
        NodeOrToken::Node(node) => node,
    };
    let code = |t: &SyntaxToken<'_>| t.owns_trivia() && !t.text().trim().is_empty();
    let first = node.tokens().find(code)?;
    let start = node.text_range().start();
    let last = std::iter::successors(node.last_token(), SyntaxToken::prev_token)
        .take_while(|t| t.text_range().start() >= start)
        .find(code)?;
    Some(TextRange::new(
        first.text_range().start(),
        last.text_range().end(),
    ))
}

/// What replaces `token` when it goes: nothing, or a space where the text
/// on each side would otherwise join, as in `if(c)`.
fn unspaced(text: &str, token: &SyntaxToken<'_>) -> String {
    let (start, end) = (
        usize::from(token.text_range().start()),
        usize::from(token.text_range().end()),
    );
    let before = text[..start]
        .chars()
        .next_back()
        .is_some_and(|c| !c.is_whitespace());
    let after = text[end..]
        .chars()
        .next()
        .is_some_and(|c| !c.is_whitespace());
    if before && after { " " } else { "" }.to_string()
}

/// A character of an operator such as `<:`, which `?` would join.
fn is_operator_char(c: char) -> bool {
    "!#%&*+-/:<=>?@\\^|~".contains(c)
}

/// `word` in place of `token`, with a space on each side where the source
/// has none, so `b=>c` doesn't become `basc`.
fn spaced(text: &str, token: &SyntaxToken<'_>, word: &str) -> String {
    let (start, end) = (
        usize::from(token.text_range().start()),
        usize::from(token.text_range().end()),
    );
    let before = text[..start]
        .chars()
        .next_back()
        .is_some_and(|c| !c.is_whitespace());
    let after = text[end..]
        .chars()
        .next()
        .is_some_and(|c| !c.is_whitespace());
    format!(
        "{}{word}{}",
        if before { " " } else { "" },
        if after { " " } else { "" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lossless_tree::parse;

    const NEW: Dialect = Dialect::SCALA3;
    const OLD: Dialect = Dialect {
        star_wildcard_import: false,
        as_for_import_rename: false,
        question_mark_type_wildcard: false,
        postfix_star_vararg_splices: false,
        quiet_syntax: false,
    };

    fn rewritten(source: &str, dialect: Dialect) -> String {
        rewrite(&parse(source), dialect).unwrap_or_else(|| source.to_string())
    }

    #[test]
    fn wildcards_and_renames() {
        assert_eq!(rewritten("import a._\n", NEW), "import a.*\n");
        assert_eq!(
            rewritten("import a.{b => c, d => _, _}\n", NEW),
            "import a.{b as c, d as _, *}\n"
        );
        assert_eq!(
            rewritten("import a.{given, _}\n", NEW),
            "import a.{given, *}\n"
        );
        assert_eq!(rewritten("export a._\n", NEW), "export a.*\n");
        assert_eq!(
            rewritten("object A {\n  import b.c._\n}\n", NEW),
            "object A {\n  import b.c.*\n}\n"
        );
    }

    #[test]
    fn already_new_or_not_a_wildcard() {
        for source in [
            "import a.*\n",
            "import a.{b as c, *}\n",
            "import a.b as c\n",
            "import a.`_`\n",
            "import a.given\n",
            "val f = (_: Int) + 1\n",
            "def g(x: Int => Int) = x\n",
        ] {
            assert_eq!(rewrite(&parse(source), NEW), None, "{source}");
        }
    }

    #[test]
    fn a_rename_without_spaces_gets_them() {
        assert_eq!(rewritten("import a.{b=>c}\n", NEW), "import a.{b as c}\n");
    }

    #[test]
    fn comments_stay() {
        assert_eq!(
            rewritten("import a.{b /* x */ => c} // y\n", NEW),
            "import a.{b /* x */ as c} // y\n"
        );
    }

    #[test]
    fn only_what_the_dialect_allows() {
        let star_only = Dialect {
            as_for_import_rename: false,
            ..NEW
        };
        assert_eq!(
            rewritten("import a.{b => c, _}\n", star_only),
            "import a.{b => c, *}\n"
        );
        assert_eq!(rewrite(&parse("import a.{b => c, _}\n"), OLD), None);
        let no_question_mark = Dialect {
            question_mark_type_wildcard: false,
            ..NEW
        };
        assert_eq!(
            rewrite(&parse("val a: List[_] = b\n"), no_question_mark),
            None
        );
    }

    #[test]
    fn back_to_scala_2() {
        let scala2 = OLD;
        assert_eq!(
            rewritten("import a.{b as c, d as _, *}\n", scala2),
            "import a.{b => c, d => _, _}\n"
        );
        assert_eq!(rewritten("import a.*\n", scala2), "import a._\n");
        assert_eq!(
            rewritten("import a.b as c, d.e as _\n", scala2),
            "import a.{b => c}, d.{e => _}\n"
        );
        assert_eq!(rewrite(&parse("import a.{b => c, _}\n"), scala2), None);
        // No Scala 2 form without a path.
        assert_eq!(rewrite(&parse("import foo as f\n"), scala2), None);
    }

    #[test]
    fn type_wildcards() {
        for (source, expected) in [
            ("val a: List[_] = b\n", "val a: List[?] = b\n"),
            ("def f(m: Map[_, Int]) = m\n", "def f(m: Map[?, Int]) = m\n"),
            ("val a: List[_ <: A] = b\n", "val a: List[? <: A] = b\n"),
            (
                "val a: List[_ >: A <: B] = b\n",
                "val a: List[? >: A <: B] = b\n",
            ),
            ("val a: List[_<:A] = b\n", "val a: List[? <:A] = b\n"),
            (
                "x match { case _: List[_] => }\n",
                "x match { case _: List[?] => }\n",
            ),
            ("val a: F[List[_]] = b\n", "val a: F[List[?]] = b\n"),
        ] {
            assert_eq!(rewritten(source, NEW), expected, "{source}");
        }
    }

    #[test]
    fn not_type_wildcards() {
        for source in [
            // Type parameters.
            "def f[F[_]](x: F[Int]) = x\n",
            "type F[_] = Int\n",
            "class C[A[_ <: B]]\n",
            // Not a type.
            "val f = g(_)\n",
            "val a: List[?] = b\n",
        ] {
            assert_eq!(rewrite(&parse(source), NEW), None, "{source}");
        }
        // Without the new syntax, `?` is a name: kind-projector's
        // placeholder here.
        assert_eq!(rewrite(&parse("val a: F[Either[A, ?]] = b\n"), OLD), None);
    }

    #[test]
    fn vararg_splices() {
        for (old, new) in [
            ("f(xs: _*)\n", "f(xs*)\n"),
            ("f(a, g(b): _*)\n", "f(a, g(b)*)\n"),
            ("new A(xs: _*)\n", "new A(xs*)\n"),
            (
                "x match { case Seq(a, xs @ _*) => }\n",
                "x match { case Seq(a, xs*) => }\n",
            ),
        ] {
            assert_eq!(rewritten(old, NEW), new, "{old}");
            assert_eq!(rewritten(new, OLD), old, "{new}");
        }
        assert_eq!(rewritten("f(xs:_ *)\n", NEW), "f(xs*)\n");
        for same in [
            "x match { case Seq(_*) => }\n",
            // `*` would join the name or operator.
            "f(foo_ : _*)\n",
            "f(a.++ : _*)\n",
            // A comment in the way.
            "f(xs /* c */ : _*)\n",
        ] {
            assert_eq!(rewrite(&parse(same), NEW), None, "{same}");
        }
        assert_eq!(rewrite(&parse("x match { case Seq(_*) => }\n"), OLD), None);
    }

    #[test]
    fn control_syntax() {
        for (old, new) in [
            ("if (c) a else b\n", "if c then a else b\n"),
            ("if (a) b else if (c) d\n", "if a then b else if c then d\n"),
            ("while (c) x\n", "while c do x\n"),
            ("for (x <- xs) f(x)\n", "for x <- xs do f(x)\n"),
            ("for (x <- xs) yield x\n", "for x <- xs yield x\n"),
            (
                "for (x <- xs; y <- ys) yield x\n",
                "for x <- xs; y <- ys yield x\n",
            ),
            (
                "def f =\n  if (c)\n    a\n  else\n    b\n",
                "def f =\n  if c then\n    a\n  else\n    b\n",
            ),
            ("if (c) {\n  a\n}\n", "if c then {\n  a\n}\n"),
        ] {
            assert_eq!(rewritten(old, NEW), new, "{old}");
            assert_eq!(rewritten(new, OLD), old, "{new}");
        }
        // Back, a comment before `then` is in the way.
        assert_eq!(
            rewritten("if (a /* x */) b\n", NEW),
            "if a /* x */ then b\n"
        );
        assert_eq!(rewrite(&parse("if a /* x */ then b\n"), OLD), None);
        // Written without spaces.
        assert_eq!(rewritten("if(c)a\n", NEW), "if c then a\n");
        assert_eq!(rewritten("while(c){x}\n", NEW), "while c do {x}\n");
        // Parentheses with `then` or `do` lose one or the other.
        assert_eq!(rewritten("if (c) then a\n", NEW), "if c then a\n");
        assert_eq!(rewritten("if (c) then a\n", OLD), "if (c) a\n");
        assert_eq!(
            rewritten("for (x <- xs) do f(x)\n", OLD),
            "for (x <- xs) f(x)\n"
        );
        // Braces stay in Scala 3, and come back in Scala 2 for a `for`
        // over several lines.
        let braced = "for {\n  x <- xs\n  y <- ys\n} yield x\n";
        assert_eq!(rewrite(&parse(braced), NEW), None);
        assert_eq!(
            rewritten("for\n  x <- xs\n  y <- ys\nyield x\n", OLD),
            braced
        );
        assert_eq!(
            rewritten("for\n  x <- xs\ndo\n  f(x)\n", OLD),
            "for {\n  x <- xs\n}\n  f(x)\n"
        );
    }

    #[test]
    fn control_syntax_left_as_written() {
        for source in [
            // A line break inside the parentheses.
            "if (\n  c\n) a\n",
            // A condition starting with `(`.
            "if ((a, b) == c) d\n",
            "do x while (c)\n",
            // What tree-sitter-scala would read differently.
            "for (x <- xs) x match {\n  case _ =>\n}\n",
            "def f =\n  if (c)\n    a\n  end if\n",
        ] {
            assert_eq!(rewrite(&parse(source), NEW), None, "{source}");
        }
        // Neither `if`: the outer one would take the `else`.
        assert_eq!(rewrite(&parse("if (a) x = if (b) 1 else 2\n"), NEW), None);
        let source3 = Dialect {
            quiet_syntax: false,
            ..NEW
        };
        assert_eq!(rewrite(&parse("if (c) a\n"), source3), None);
        for source in [
            "def f =\n  if c then\n    a\n  end if\n",
            "inline if c then a\n",
        ] {
            assert_eq!(rewrite(&parse(source), OLD), None, "{source}");
        }
    }

    #[test]
    fn format_off_is_left_alone() {
        let source = "// format: off\nimport a._\n// format: on\nimport b._\n";
        assert_eq!(
            rewritten(source, NEW),
            "// format: off\nimport a._\n// format: on\nimport b.*\n"
        );
    }
}
