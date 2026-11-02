//! Re-indentation by uniform shifts.
//!
//! A container is a place whose statements sit on their own lines: a block
//! in braces, a Scala 3 indentation region, or the whole file. Its
//! statements belong 2 spaces deeper than the line that opens it (the line
//! with the `{`, or the line before a region), or at column 0 for the file.
//! Every line inside a container moves by the same amount, so a statement
//! keeps the shape of its continuation lines, and a nested container is then
//! placed from its opener's new position.
//!
//! A container whose statements aren't each on their own line at one
//! indentation moves with the container around it instead. Files with tabs
//! in their indentation, or with code that is copied verbatim for its
//! indentation (`format: off`, syntax errors, unsafe regions), keep theirs.

use std::collections::HashMap;

use crate::T;
use crate::lossless_tree::{
    NodeOrToken, SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize, Tree,
    VerbatimReason,
};

/// How many spaces each container's statements are indented past its
/// opener's line.
const STEP: usize = 2;

/// New indentation for lines, keyed by the offset of the line's first
/// non-blank character. Lines that keep their indentation aren't listed.
#[derive(Default, Debug)]
pub(super) struct Reindent {
    widths: HashMap<TextSize, usize>,
}

impl Reindent {
    /// `relaid` lists the `(` and `)` of lists whose line breaks the
    /// formatter decides: what follows the `)` ends up on a line indented
    /// as the `(`'s line, whether the list is joined or broken.
    pub(super) fn new(tree: &Tree, relaid: &[(TextSize, TextSize)]) -> Reindent {
        compute(tree, relaid).unwrap_or_default()
    }

    /// The new indentation of the line whose first non-blank character is
    /// at `offset`, if it changes.
    pub(super) fn width(&self, offset: TextSize) -> Option<usize> {
        self.widths.get(&offset).copied()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.widths.is_empty()
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.widths.len()
    }
}

struct Container<'t> {
    /// Where its statements can be: between the braces, or the region.
    inner: TextRange,
    statements: Vec<SyntaxElement<'t>>,
    /// The token whose line the statements are indented from. `None` for the
    /// file.
    opener: Option<SyntaxToken<'t>>,
    /// Scala 3 `case` clauses, which may sit at the width of their `match`.
    cases: bool,
}

/// A line that starts with code or a comment.
struct Line {
    /// Its first non-blank character.
    first: TextSize,
    /// Its indentation, in spaces.
    width: usize,
    /// Whether it starts with a comment.
    comment: bool,
}

fn compute(tree: &Tree, relaid: &[(TextSize, TextSize)]) -> Option<Reindent> {
    let text = tree.syntax().text();
    let copied_for_layout = tree.verbatim_ranges().iter().any(|v| {
        matches!(
            v.reason,
            VerbatimReason::UnsafeIndentation(_)
                | VerbatimReason::FormatOff
                | VerbatimReason::SyntaxError
        )
    });
    if copied_for_layout {
        return None;
    }

    let mut lines = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        let content = line.trim_start_matches([' ', '\t']);
        let indent = &line[..line.len() - content.len()];
        let line_start = TextSize::try_from(start).ok()?;
        start += line.len();
        if content.trim().is_empty() {
            continue;
        }
        // A line inside a multi-line string or comment is content.
        if tree
            .verbatim_at(line_start)
            .is_some_and(|v| v.range.start() < line_start)
        {
            continue;
        }
        if indent.contains('\t') {
            return None;
        }
        lines.push(Line {
            first: line_start + TextSize::try_from(indent.len()).ok()?,
            width: indent.len(),
            comment: content.starts_with("//") || content.starts_with("/*"),
        });
    }
    let line_of = |offset: TextSize| lines.partition_point(|l| l.first <= offset).checked_sub(1);
    // The line of a relaid list's `)` stands for the line of its `(`.
    let mut moved = HashMap::new();
    for &(open, close) in relaid {
        if let (Some(open), Some(close)) = (line_of(open), line_of(close))
            && open != close
        {
            moved.insert(close, open);
        }
    }
    let opener_line = |mut line: usize| {
        while let Some(&to) = moved.get(&line) {
            line = to;
        }
        line
    };

    let mut containers = vec![Container {
        inner: tree.syntax().text_range(),
        statements: tree.syntax().non_trivia_children().collect(),
        opener: None,
        cases: false,
    }];
    for node in tree.syntax().descendants() {
        if let Some(container) = container(node) {
            containers.push(container);
        }
    }
    // Outer containers first, so each one's opener line is placed before it.
    containers.sort_by_key(|c| (c.inner.start(), std::cmp::Reverse(c.inner.end())));

    // The innermost container of each line: later, inner ones overwrite.
    let mut owner = vec![0; lines.len()];
    let mut parent = vec![0; containers.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (index, container) in containers.iter().enumerate() {
        while stack
            .last()
            .is_some_and(|&outer| containers[outer].inner.end() <= container.inner.start())
        {
            stack.pop();
        }
        parent[index] = stack.last().copied().unwrap_or(0);
        stack.push(index);
        let from = lines.partition_point(|l| l.first < container.inner.start());
        let to = lines.partition_point(|l| l.first < container.inner.end());
        for o in &mut owner[from..to] {
            *o = index;
        }
    }

    let mut shift = vec![0isize; containers.len()];
    // For a container placed on its own: its statements' old and new
    // indentation.
    let mut placed: Vec<Option<(usize, usize)>> = vec![None; containers.len()];
    for (index, container) in containers.iter().enumerate() {
        let inherited = if index == 0 { 0 } else { shift[parent[index]] };
        shift[index] = inherited;
        let Some(indent) = statement_indent(container) else {
            continue;
        };
        // The lines a container owns sit deeper than its opener's line; at
        // the same width only for cases written at their `match`'s width.
        // Anything else means tree-sitter-scala read the layout differently
        // from scalac, as a leading `||` taken into the last case.
        let mut floor = 0;
        let target = match container.opener {
            None => 0,
            Some(opener) => {
                let Some(line) = line_of(opener.text_range().start()).map(opener_line) else {
                    continue;
                };
                let width = lines[line].width as isize + shift[owner[line]];
                if width < 0 {
                    continue;
                }
                // Cases written at the width of their `match` stay there, as
                // scalafmt leaves them.
                let step = if container.cases && indent == lines[line].width {
                    0
                } else {
                    STEP
                };
                floor = lines[line].width + step;
                width as usize + step
            }
        };
        let candidate = target as isize - indent as isize;
        // Every line of the container must stay at column 0 or deeper.
        // Comment lines don't count: they're placed with the statements.
        let fits = owner
            .iter()
            .zip(&lines)
            .filter(|(o, l)| **o == index && !l.comment)
            .all(|(_, l)| l.width >= floor && l.width as isize + candidate >= 0);
        // A line right after the container that was shallower than its
        // statements stays shallower: in `xs.map: x =>`, the body, then
        // `.filter(p)` between the two, a body moved to the `.filter` width
        // would take it in.
        let next = lines.partition_point(|l| l.first < container.inner.end());
        let keeps_next = lines.get(next).is_none_or(|l| {
            l.comment || l.width >= indent || l.width as isize + inherited < target as isize
        });
        if fits && keeps_next {
            shift[index] = candidate;
            placed[index] = Some((indent, target));
        }
    }

    let mut widths = HashMap::new();
    for (line, &o) in lines.iter().zip(&owner) {
        let width = match placed[o] {
            // A comment on its own line, shallower than the statements
            // around it, moves to their indentation, as in scalafmt.
            Some((indent, target)) if line.comment && line.width < indent && shift[o] != 0 => {
                target
            }
            // A container that inherits a shift can push a line past column
            // 0; the file keeps its indentation then.
            _ => usize::try_from(line.width as isize + shift[o]).ok()?,
        };
        if width != line.width {
            widths.insert(line.first, width);
        }
    }
    Some(Reindent { widths })
}

/// The container a node makes, if any.
fn container(node: SyntaxNode<'_>) -> Option<Container<'_>> {
    // A lambda body in braces is no region, since its indentation doesn't
    // matter, but its lines are indented from its arrow all the same.
    let lambda_body = node.kind() == SyntaxKind::IndentedBlock
        && node
            .parent()
            .is_some_and(|p| p.kind() == SyntaxKind::LambdaExpression);
    if node.is_indentation_region() || lambda_body {
        let statements: Vec<_> = if lambda_body {
            node.non_trivia_children()
                .filter(|c| c.kind().is_named() && c.kind() != SyntaxKind::EndMarker)
                .collect()
        } else {
            node.region_statements().collect()
        };
        let first = first_token(statements.first()?)?;
        let opener = std::iter::successors(first.prev_token(), SyntaxToken::prev_token)
            .find(SyntaxToken::owns_trivia)?;
        // From after the opener, which an `extension`'s range includes, to
        // its last token: the node can end at the next line's first token.
        let last = node.tokens().filter(SyntaxToken::owns_trivia).last()?;
        return Some(Container {
            inner: TextRange::new(opener.text_range().end(), last.text_range().end()),
            statements,
            opener: Some(definition_start(node).unwrap_or(opener)),
            cases: node.kind() == SyntaxKind::IndentedCases,
        });
    }
    let mut children: Vec<_> = node.non_trivia_children().collect();
    // A match type's braces come after its scrutinee and `match`.
    if node.kind() == SyntaxKind::MatchType {
        let open = children.iter().position(|c| c.kind() == T!["{"])?;
        children.drain(..open);
    }
    let (NodeOrToken::Token(open), NodeOrToken::Token(close)) =
        (children.first()?, children.last()?)
    else {
        return None;
    };
    if open.kind() != T!["{"] || close.kind() != T!["}"] {
        return None;
    }
    let statements = children[1..children.len() - 1]
        .iter()
        .filter(|c| c.kind().is_named())
        .copied()
        .collect();
    Some(Container {
        inner: TextRange::new(open.text_range().end(), close.text_range().start()),
        statements,
        opener: Some(definition_start(node).unwrap_or(*open)),
        cases: false,
    })
}

/// For the body of a definition, a class body or `case` clauses, the first
/// token of what it belongs to. A body is indented from that first line, as in scalafmt, even
/// when the header goes on over more lines: `extends` and `with` clauses, a
/// result type, or parameters.
fn definition_start(body: SyntaxNode<'_>) -> Option<SyntaxToken<'_>> {
    let owner = body.parent()?;
    // Also `case` clauses, from the start of their `match`, which may be on
    // an earlier line than the `match` keyword.
    let starts_owner = matches!(
        body.kind(),
        SyntaxKind::TemplateBody
            | SyntaxKind::EnumBody
            | SyntaxKind::WithTemplateBody
            | SyntaxKind::IndentedCases
    );
    if !starts_owner && !owner.kind().name().ends_with("_definition") {
        return None;
    }
    owner.tokens().find(SyntaxToken::owns_trivia)
}

/// The indentation the container's statements share, if each starts its own
/// line at the same width. `None` for a container whose layout is left to
/// the one around it, including one with no statements.
fn statement_indent(container: &Container<'_>) -> Option<usize> {
    let mut indent = None;
    for statement in &container.statements {
        let first = first_token(statement)?;
        // tree-sitter-scala reads `end Name` after a body holding only a
        // comment as a postfix call. Moving it would make it close something
        // else, and the tree wouldn't show it.
        if !first.is_first_on_line() || first.text() == "end" {
            return None;
        }
        let width = first.indentation().len();
        if *indent.get_or_insert(width) != width {
            return None;
        }
    }
    indent
}

fn first_token<'t>(element: &SyntaxElement<'t>) -> Option<SyntaxToken<'t>> {
    match element {
        NodeOrToken::Token(token) => Some(*token).filter(SyntaxToken::owns_trivia),
        NodeOrToken::Node(node) => node.tokens().find(SyntaxToken::owns_trivia),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lossless_tree::parse;

    /// The source with each line moved to its new indentation.
    fn reindented(source: &str) -> String {
        let tree = parse(source);
        let reindent = Reindent::new(&tree, &[]);
        let mut out = String::new();
        let mut start = 0;
        for line in source.split_inclusive('\n') {
            let content = line.trim_start_matches(' ');
            let first = TextSize::try_from(start + line.len() - content.len()).unwrap();
            match reindent.width(first) {
                Some(width) => {
                    out.push_str(&" ".repeat(width));
                    out.push_str(content);
                }
                None => out.push_str(line),
            }
            start += line.len();
        }
        out
    }

    #[test]
    fn braces_get_two_spaces_per_level() {
        let source = "object A {\n    def f = {\n            1\n    }\n}\n";
        assert_eq!(
            reindented(source),
            "object A {\n  def f = {\n    1\n  }\n}\n"
        );
    }

    #[test]
    fn continuation_lines_move_with_their_statement() {
        let source = "object A {\n    val x = f(\n        a,\n        b\n    )\n}\n";
        assert_eq!(
            reindented(source),
            "object A {\n  val x = f(\n      a,\n      b\n  )\n}\n"
        );
    }

    #[test]
    fn scala3_regions_are_placed_from_their_opener() {
        let source = "object A:\n    def f =\n        1\n";
        assert_eq!(reindented(source), "object A:\n  def f =\n    1\n");
    }

    #[test]
    fn already_formatted_code_stays() {
        let source = "object A {\n  def f = {\n    g(\n      a\n    )\n  }\n}\n";
        assert_eq!(Reindent::new(&parse(source), &[]).len(), 0);
    }

    #[test]
    fn a_continuation_after_a_body_stays_outside_it() {
        // Moving the body to 4 would put `.map` inside it.
        let source = "object O:\n  val q = xs.filter: x =>\n      x > 1\n    .map: x =>\n      x\n";
        let out = reindented(source);
        assert!(out.contains("\n      x > 1\n    .map"), "{out}");
    }

    #[test]
    fn an_end_marker_read_as_a_statement_stays() {
        let source = "trait A:\n//\nend A\n";
        assert_eq!(Reindent::new(&parse(source), &[]).len(), 0);
    }

    #[test]
    fn mixed_statement_indentation_moves_with_the_parent() {
        // The inner block's statements disagree, so it keeps its shape and
        // moves with the object's body.
        let source = "object A {\n    def f = {\n        1\n          2\n    }\n}\n";
        assert_eq!(
            reindented(source),
            "object A {\n  def f = {\n      1\n        2\n  }\n}\n"
        );
    }
}
