//! Attaches whitespace and comments ("trivia") to the tokens around them.
//!
//! tree-sitter puts comments wherever they fall in the tree, so a comment's
//! position says nothing about which code it belongs to. Here each piece of
//! trivia gets exactly one owner token:
//!
//! - Trailing trivia: everything after a token up to the end of its line,
//!   such as `// note` in `x, // note`.
//! - Leading trivia: everything from that newline up to the next token,
//!   including blank lines, comments on their own lines, and indentation.
//! - Trivia after the last token's line belongs to the end of the file.
//!
//! Missing tokens (zero-width, from error recovery) never own trivia.
//! Joining each token's leading trivia, text and trailing trivia, then the
//! end-of-file trivia, gives back the source.

use std::fmt;

use super::{
    Child, NodeId, NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, TextSize, Tree,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum TriviaKind {
    /// Spaces, tabs and other non-newline whitespace.
    Whitespace,
    /// Exactly one line break: `\n`, `\r\n` or `\r`.
    Newline,
    /// `// ...`, not including the line break.
    LineComment,
    /// `/* ... */`, including doc comments.
    BlockComment,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct TriviaPiece {
    kind: TriviaKind,
    range: TextRange,
}

/// A range of indexes into `Tree::trivia`.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(super) struct TriviaSpan {
    start: u32,
    end: u32,
}

impl TriviaSpan {
    fn new(start: usize, end: usize) -> Self {
        let index = |i: usize| u32::try_from(i).expect("fewer trivia pieces than source bytes");
        TriviaSpan {
            start: index(start),
            end: index(end),
        }
    }
}

/// One piece of whitespace or one comment.
#[derive(Clone, Copy)]
pub struct SyntaxTrivia<'t> {
    tree: &'t Tree,
    piece: &'t TriviaPiece,
}

impl<'t> SyntaxTrivia<'t> {
    pub fn kind(&self) -> TriviaKind {
        self.piece.kind
    }

    pub fn text_range(&self) -> TextRange {
        self.piece.range
    }

    pub fn text(&self) -> &'t str {
        &self.tree.text[self.piece.range]
    }

    pub fn is_comment(&self) -> bool {
        matches!(
            self.kind(),
            TriviaKind::LineComment | TriviaKind::BlockComment
        )
    }

    /// A Scaladoc comment: `/** ... */`.
    pub fn is_doc_comment(&self) -> bool {
        self.kind() == TriviaKind::BlockComment
            && self.text().starts_with("/**")
            && self.text() != "/**/"
    }
}

impl fmt::Debug for SyntaxTrivia<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}@{:?} {:?}",
            self.kind(),
            self.text_range(),
            self.text()
        )
    }
}

impl fmt::Display for SyntaxTrivia<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}

impl Tree {
    fn trivia_in(
        &self,
        span: TriviaSpan,
    ) -> impl DoubleEndedIterator<Item = SyntaxTrivia<'_>> + ExactSizeIterator + Clone + '_ {
        self.trivia[span.start as usize..span.end as usize]
            .iter()
            .map(move |piece| SyntaxTrivia { tree: self, piece })
    }

    /// Trivia after the last token's line: comments and blank lines at the
    /// end of the file. For a file with no code, all of its trivia.
    pub fn eof_trivia(
        &self,
    ) -> impl DoubleEndedIterator<Item = SyntaxTrivia<'_>> + ExactSizeIterator + Clone + '_ {
        self.trivia_in(self.eof_trivia)
    }
}

impl<'t> SyntaxToken<'t> {
    /// Whitespace and comments before this token that belong to it. Empty for
    /// trivia and missing tokens.
    pub fn leading_trivia(
        &self,
    ) -> impl DoubleEndedIterator<Item = SyntaxTrivia<'t>> + ExactSizeIterator + Clone + use<'t>
    {
        self.tree.trivia_in(self.data().leading)
    }

    /// Whitespace and comments after this token, up to the end of its line.
    /// Empty for trivia and missing tokens.
    pub fn trailing_trivia(
        &self,
    ) -> impl DoubleEndedIterator<Item = SyntaxTrivia<'t>> + ExactSizeIterator + Clone + use<'t>
    {
        self.tree.trivia_in(self.data().trailing)
    }

    /// The next token in the file, including trivia tokens.
    pub fn next_token(&self) -> Option<SyntaxToken<'t>> {
        let mut element = NodeOrToken::Token(*self);
        loop {
            match element.next_sibling_or_token() {
                Some(sibling) => {
                    let first = match sibling {
                        NodeOrToken::Token(token) => Some(token),
                        NodeOrToken::Node(node) => node.first_token(),
                    };
                    if first.is_some() {
                        return first;
                    }
                    element = sibling; // An empty node; keep going.
                }
                None => element = NodeOrToken::Node(element.parent()?),
            }
        }
    }

    /// The previous token in the file, including trivia tokens.
    pub fn prev_token(&self) -> Option<SyntaxToken<'t>> {
        let mut element = NodeOrToken::Token(*self);
        loop {
            match element.prev_sibling_or_token() {
                Some(sibling) => {
                    let last = match sibling {
                        NodeOrToken::Token(token) => Some(token),
                        NodeOrToken::Node(node) => node.last_token(),
                    };
                    if last.is_some() {
                        return last;
                    }
                    element = sibling;
                }
                None => element = NodeOrToken::Node(element.parent()?),
            }
        }
    }

    /// Code tokens own trivia; whitespace, comments and missing tokens don't.
    pub(crate) fn owns_trivia(&self) -> bool {
        !self.is_trivia() && !self.is_missing()
    }
}

impl<'t> SyntaxNode<'t> {
    /// The first token in this subtree, which may be trivia.
    pub fn first_token(&self) -> Option<SyntaxToken<'t>> {
        self.tokens().next()
    }

    /// The last token in this subtree, which may be trivia.
    pub fn last_token(&self) -> Option<SyntaxToken<'t>> {
        let mut element = self.last_child_or_token()?;
        loop {
            match element {
                NodeOrToken::Token(token) => return Some(token),
                NodeOrToken::Node(node) => {
                    if let Some(child) = node.last_child_or_token() {
                        element = child;
                        continue;
                    }
                }
            }
            // An empty node: step back, without leaving this subtree.
            loop {
                if let Some(prev) = element.prev_sibling_or_token() {
                    element = prev;
                    break;
                }
                let parent = element.parent()?;
                if parent == *self {
                    return None;
                }
                element = NodeOrToken::Node(parent);
            }
        }
    }

    /// The leading trivia of this node's first code token, such as the doc
    /// comment above a definition.
    pub fn leading_trivia(
        &self,
    ) -> impl DoubleEndedIterator<Item = SyntaxTrivia<'t>> + ExactSizeIterator + Clone + use<'t>
    {
        let span = self
            .tokens()
            .find(SyntaxToken::owns_trivia)
            .map_or_else(TriviaSpan::default, |token| token.data().leading);
        self.tree.trivia_in(span)
    }

    /// The trailing trivia of this node's last code token, such as a comment
    /// at the end of the line.
    pub fn trailing_trivia(
        &self,
    ) -> impl DoubleEndedIterator<Item = SyntaxTrivia<'t>> + ExactSizeIterator + Clone + use<'t>
    {
        let start = self.text_range().start();
        let owner = std::iter::successors(self.last_token(), SyntaxToken::prev_token)
            .take_while(|token| token.text_range().start() >= start)
            .find(SyntaxToken::owns_trivia);
        let span = owner.map_or_else(TriviaSpan::default, |token| token.data().trailing);
        self.tree.trivia_in(span)
    }
}

// ---------------------------------------------------------------------------
// Assigning owners.

enum Item {
    Owner { parent: NodeId, index: usize },
    Trivia(TriviaPiece),
}

/// Splits the file into code tokens and trivia pieces, then gives each run of
/// trivia between two code tokens to its owners. Called once while parsing.
pub(super) fn attach(parse: &mut Tree) {
    let items = lex(parse.syntax());

    let mut trivia = Vec::new();
    let mut owners: Vec<(NodeId, usize, TriviaSpan, TriviaSpan)> = Vec::new();
    let mut run_start = 0;

    // The trivia since the previous owner splits at its first newline:
    // before it trails the previous owner, the rest leads the next one.
    let split = |trivia: &[TriviaPiece], run_start: usize, has_previous: bool| {
        if !has_previous {
            return run_start;
        }
        trivia[run_start..]
            .iter()
            .position(|piece| piece.kind == TriviaKind::Newline)
            .map_or(trivia.len(), |i| run_start + i)
    };

    for item in items {
        match item {
            Item::Trivia(piece) => trivia.push(piece),
            Item::Owner { parent, index } => {
                let at = split(&trivia, run_start, !owners.is_empty());
                if let Some(previous) = owners.last_mut() {
                    previous.3 = TriviaSpan::new(run_start, at);
                }
                owners.push((
                    parent,
                    index,
                    TriviaSpan::new(at, trivia.len()),
                    TriviaSpan::default(),
                ));
                run_start = trivia.len();
            }
        }
    }
    let at = split(&trivia, run_start, !owners.is_empty());
    if let Some(previous) = owners.last_mut() {
        previous.3 = TriviaSpan::new(run_start, at);
    }
    parse.eof_trivia = TriviaSpan::new(at, trivia.len());
    parse.trivia = trivia;

    for (parent, index, leading, trailing) in owners {
        match &mut parse.nodes[parent].children[index] {
            Child::Token(token) => {
                token.leading = leading;
                token.trailing = trailing;
            }
            Child::Node(_) => unreachable!("owners are tokens"),
        }
    }
}

fn lex(root: SyntaxNode<'_>) -> Vec<Item> {
    let text = &root.tree.text;
    let mut items = Vec::new();
    let mut last_comment = None;
    for token in root.tokens() {
        if token.owns_trivia() {
            items.push(Item::Owner {
                parent: token.parent,
                index: token.index,
            });
        } else if token.is_extra() {
            // Comments can be nodes with several tokens; emit each once.
            let range = comment_range(token);
            if last_comment != Some(range) {
                last_comment = Some(range);
                // tree-sitter-scala sometimes includes neighboring spaces in
                // a comment, depending on the code around it. Split them off
                // so a comment's text doesn't change with its surroundings.
                let comment = &text[range];
                let body = comment.trim();
                let body_start = range.start() + offset(comment.len() - comment.trim_start().len());
                let body_range = TextRange::at(body_start, offset(body.len()));
                let before = TextRange::new(range.start(), body_range.start());
                let after = TextRange::new(body_range.end(), range.end());
                split_whitespace(before, &text[before], &mut items);
                let kind = if body.starts_with("//") {
                    TriviaKind::LineComment
                } else {
                    TriviaKind::BlockComment
                };
                items.push(Item::Trivia(TriviaPiece {
                    kind,
                    range: body_range,
                }));
                split_whitespace(after, &text[after], &mut items);
            }
        } else if token.kind() == SyntaxKind::Whitespace {
            split_whitespace(token.text_range(), &text[token.text_range()], &mut items);
        }
        // Anything else is a missing token: zero-width, no trivia.
    }
    items
}

/// The whole comment a comment token belongs to.
fn comment_range(token: SyntaxToken<'_>) -> TextRange {
    let mut range = token.text_range();
    let mut node = token.parent_node();
    while node.is_extra() {
        range = node.text_range();
        match node.parent() {
            Some(parent) => node = parent,
            None => break,
        }
    }
    range
}

fn split_whitespace(range: TextRange, text: &str, items: &mut Vec<Item>) {
    let base = usize::from(range.start());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let (kind, len) = match bytes[i] {
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => (TriviaKind::Newline, 2),
            b'\n' | b'\r' => (TriviaKind::Newline, 1),
            _ => {
                let len = text[i..].find(['\n', '\r']).unwrap_or(text.len() - i);
                (TriviaKind::Whitespace, len)
            }
        };
        let piece_range = TextRange::at(offset(base + i), offset(len));
        // Adjacent whitespace tokens become one piece.
        if let (TriviaKind::Whitespace, Some(Item::Trivia(last))) = (kind, items.last_mut())
            && last.kind == TriviaKind::Whitespace
            && last.range.end() == piece_range.start()
        {
            last.range = last.range.cover(piece_range);
        } else {
            items.push(Item::Trivia(TriviaPiece {
                kind,
                range: piece_range,
            }));
        }
        i += len;
    }
}

fn offset(n: usize) -> TextSize {
    TextSize::try_from(n).expect("parse() checks the source is under 4 GiB")
}

#[cfg(test)]
pub(super) fn reassemble(parse: &Tree) -> String {
    let mut out = String::new();
    for token in parse.syntax().tokens().filter(SyntaxToken::owns_trivia) {
        out.extend(token.leading_trivia().map(|t| t.text()));
        out.push_str(token.text());
        out.extend(token.trailing_trivia().map(|t| t.text()));
    }
    out.extend(parse.eof_trivia().map(|t| t.text()));
    out
}

#[cfg(test)]
mod tests {
    use super::super::parse;
    use super::*;

    fn token<'t>(parse: &'t Tree, text: &str) -> SyntaxToken<'t> {
        let mut matches = parse
            .syntax()
            .tokens()
            .filter(|t| t.text() == text && t.owns_trivia());
        let token = matches
            .next()
            .unwrap_or_else(|| panic!("no token {text:?}"));
        assert!(matches.next().is_none(), "several tokens {text:?}");
        token
    }

    fn comments<'t>(trivia: impl Iterator<Item = SyntaxTrivia<'t>>) -> Vec<&'t str> {
        trivia
            .filter(SyntaxTrivia::is_comment)
            .map(|t| t.text())
            .collect()
    }

    fn kinds<'t>(trivia: impl Iterator<Item = SyntaxTrivia<'t>>) -> Vec<TriviaKind> {
        trivia.map(|t| t.kind()).collect()
    }

    const PARAMS: &str = "def f(\n  a: Int, // first\n  b: Int\n) = a +\n  // why\n  b\n";

    #[test]
    fn same_line_comment_trails_previous_token() {
        let tree = parse(PARAMS);
        assert_eq!(comments(token(&tree, ",").trailing_trivia()), ["// first"]);
        let b = tree.syntax().tokens().find(|t| t.text() == "b").unwrap();
        assert!(comments(b.leading_trivia()).is_empty());
    }

    #[test]
    fn own_line_comment_leads_next_token() {
        let tree = parse(PARAMS);
        let last_b = tree
            .syntax()
            .tokens()
            .filter(|t| t.text() == "b")
            .last()
            .unwrap();
        assert_eq!(comments(last_b.leading_trivia()), ["// why"]);
        assert_eq!(
            kinds(last_b.leading_trivia()),
            [
                TriviaKind::Newline,
                TriviaKind::Whitespace,
                TriviaKind::LineComment,
                TriviaKind::Newline,
                TriviaKind::Whitespace
            ]
        );
        assert!(comments(token(&tree, "+").trailing_trivia()).is_empty());
    }

    #[test]
    fn block_comment_on_same_line_trails() {
        let tree = parse("val x = a /* why */ + b");
        assert_eq!(comments(token(&tree, "a").trailing_trivia()), ["/* why */"]);
    }

    #[test]
    fn file_start_and_end() {
        let tree = parse("// header\n\nobject A // same line\n// footer\n");
        assert_eq!(
            comments(token(&tree, "object").leading_trivia()),
            ["// header"]
        );
        assert_eq!(
            comments(token(&tree, "A").trailing_trivia()),
            ["// same line"]
        );
        assert_eq!(comments(tree.eof_trivia()), ["// footer"]);
    }

    #[test]
    fn file_without_code_is_all_eof_trivia() {
        let tree = parse("// only\n/* comments */\n");
        assert_eq!(comments(tree.eof_trivia()), ["// only", "/* comments */"]);
        assert_eq!(reassemble(&tree), tree.syntax().text());
    }

    #[test]
    fn blank_lines_are_countable() {
        let tree = parse("val a = 1\n\n\nval b = 2");
        let newlines = token(&tree, "b")
            .parent()
            .unwrap()
            .leading_trivia()
            .filter(|t| t.kind() == TriviaKind::Newline)
            .count();
        assert_eq!(newlines, 3);
    }

    #[test]
    fn doc_comment_leads_definition() {
        let tree = parse("/** Docs. */\n@deprecated\nclass A /* not docs */");
        let class = tree.syntax().children().next().unwrap();
        let docs: Vec<_> = class
            .leading_trivia()
            .filter(SyntaxTrivia::is_doc_comment)
            .collect();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].text(), "/** Docs. */");
        assert_eq!(comments(class.trailing_trivia()), ["/* not docs */"]);
    }

    #[test]
    fn node_trailing_trivia_skips_trailing_whitespace_tokens() {
        let tree = parse("object A:\n  val x = 1 // note\n");
        let val = tree
            .syntax()
            .descendants()
            .find(|n| n.kind() == SyntaxKind::ValDefinition)
            .unwrap();
        assert_eq!(comments(val.trailing_trivia()), ["// note"]);
    }

    #[test]
    fn error_recovery_code_is_not_a_comment() {
        // tree-sitter flags the ERROR node holding `=\n}` as "extra".
        let tree = parse("object A {\n  def f: Int =\n}\nobject B\n");
        assert!(!tree.errors().is_empty());
        let comments: Vec<_> = tree
            .syntax()
            .tokens()
            .flat_map(|t| t.leading_trivia().chain(t.trailing_trivia()))
            .filter(SyntaxTrivia::is_comment)
            .collect();
        assert!(comments.is_empty(), "{comments:?}");
        assert!(
            tree.syntax()
                .tokens()
                .any(|t| t.text() == "}" && !t.is_trivia())
        );
    }

    #[test]
    fn spaces_tree_sitter_puts_in_a_comment_are_split_off() {
        // tree-sitter-scala parses the second comment as "/*c*/ ".
        let source = "class A {\n  /*c*/ def a = x\n  /*c*/ def b = x\n}\n";
        let tree = parse(source);
        let in_tree: Vec<_> = tree
            .syntax()
            .tokens()
            .filter(|t| t.kind() == SyntaxKind::BlockComment)
            .map(|t| t.text())
            .collect();
        assert_eq!(
            in_tree,
            ["/*c*/", "/*c*/ "],
            "tree-sitter changed; this test may be moot"
        );
        let defs: Vec<_> = tree
            .syntax()
            .descendants()
            .filter(|n| n.kind() == SyntaxKind::FunctionDefinition)
            .collect();
        for def in defs {
            assert_eq!(comments(def.leading_trivia()), ["/*c*/"]);
            let last = def.leading_trivia().last().unwrap();
            assert_eq!(last.kind(), TriviaKind::Whitespace);
        }
        assert_eq!(reassemble(&tree), source);
    }

    #[test]
    fn using_directive_comment_is_one_piece() {
        let tree = parse("//> using scala 3.5\nobject A\n");
        assert_eq!(
            comments(token(&tree, "object").leading_trivia()),
            ["//> using scala 3.5"]
        );
    }

    #[test]
    fn crlf_is_one_newline() {
        let tree = parse("val a = 1\r\nval b = 2\r\n");
        let mut vals = tree.syntax().tokens().filter(|t| t.text() == "val");
        assert_eq!(vals.next().unwrap().leading_trivia().len(), 0);
        let second_val = vals.next().unwrap();
        let texts: Vec<_> = second_val.leading_trivia().map(|t| t.text()).collect();
        assert_eq!(texts, ["\r\n"]);
    }

    #[test]
    fn missing_tokens_own_nothing() {
        let tree = parse("object A {\n  val x = 1 // c\n");
        for token in tree.syntax().tokens().filter(SyntaxToken::is_missing) {
            assert_eq!(
                token.leading_trivia().len() + token.trailing_trivia().len(),
                0
            );
        }
        assert_eq!(comments(token(&tree, "1").trailing_trivia()), ["// c"]);
    }

    #[test]
    fn next_and_prev_token_visit_every_token() {
        let tree = parse("class A { def f(x: Int) = x /* c */ }\n");
        let all: Vec<_> = tree.syntax().tokens().collect();
        let forward: Vec<_> =
            std::iter::successors(tree.syntax().first_token(), SyntaxToken::next_token).collect();
        assert_eq!(forward, all);
        let mut backward: Vec<_> =
            std::iter::successors(tree.syntax().last_token(), SyntaxToken::prev_token).collect();
        backward.reverse();
        assert_eq!(backward, all);
    }
}
