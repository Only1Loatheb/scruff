//! A lossless syntax tree for Scala, built on tree-sitter-scala.
//!
//! Every byte of the source belongs to exactly one token, including
//! whitespace and comments, so joining all tokens gives back the source.
//! Syntax errors become `ERROR` nodes and missing tokens; parsing never fails.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

pub use text_size::{TextRange, TextSize};
use tree_sitter::{Language, Parser};
pub use trivia::{SyntaxTrivia, TriviaKind};

mod trivia;

/// The tree-sitter-scala language, loaded once.
pub fn language() -> &'static Language {
    static LANGUAGE: OnceLock<Language> = OnceLock::new();
    LANGUAGE.get_or_init(|| tree_sitter_scala::LANGUAGE.into())
}

/// The kind of a node or token, such as `class_definition`, `identifier` or
/// `"{"`. Wraps a raw `u16`: the tree-sitter symbol id, or one of the kinds
/// this module adds.
///
/// Kinds compare equal to their name: `node.kind() == "class_definition"`.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SyntaxKind(pub u16);

impl SyntaxKind {
    /// tree-sitter's built-in error symbol.
    pub const ERROR: SyntaxKind = SyntaxKind(u16::MAX);
    /// Whitespace between tokens. tree-sitter skips it; we keep it.
    pub const WHITESPACE: SyntaxKind = SyntaxKind(u16::MAX - 1);
    /// Source text inside a node that tree-sitter didn't assign to any child,
    /// such as the literal parts of an interpolated string.
    pub const TEXT: SyntaxKind = SyntaxKind(u16::MAX - 2);

    pub fn name(self) -> &'static str {
        match self {
            SyntaxKind::ERROR => "ERROR",
            SyntaxKind::WHITESPACE => "whitespace",
            SyntaxKind::TEXT => "text",
            SyntaxKind(id) => language().node_kind_for_id(id).unwrap_or("?"),
        }
    }

    /// Named kinds (`identifier`, `class_definition`) versus anonymous ones,
    /// which are literal keywords and punctuation (`"class"`, `"{"`).
    pub fn is_named(self) -> bool {
        match self {
            SyntaxKind::ERROR | SyntaxKind::WHITESPACE | SyntaxKind::TEXT => true,
            SyntaxKind(id) => language().node_kind_is_named(id),
        }
    }
}

impl PartialEq<str> for SyntaxKind {
    fn eq(&self, other: &str) -> bool {
        self.name() == other
    }
}

impl PartialEq<&str> for SyntaxKind {
    fn eq(&self, other: &&str) -> bool {
        self.name() == *other
    }
}

impl fmt::Debug for SyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_named() {
            f.write_str(self.name())
        } else {
            write!(f, "{:?}", self.name())
        }
    }
}

impl fmt::Display for SyntaxKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

// ---------------------------------------------------------------------------
// Storage. The whole tree lives in one `Vec`; handles are indexes into it.

type NodeId = usize;

struct NodeData {
    kind: SyntaxKind,
    field_name: Option<&'static str>,
    range: TextRange,
    parent: Option<NodeId>,
    /// Position among the parent's children.
    index: usize,
    extra: bool,
    children: Vec<Child>,
}

enum Child {
    Node(NodeId),
    Token(TokenData),
}

struct TokenData {
    kind: SyntaxKind,
    field_name: Option<&'static str>,
    range: TextRange,
    extra: bool,
    missing: bool,
    leading: trivia::TriviaSpan,
    trailing: trivia::TriviaSpan,
}

// ---------------------------------------------------------------------------
// Parsing.

/// The result of parsing: the tree plus any syntax errors.
pub struct Parse {
    text: String,
    nodes: Vec<NodeData>,
    errors: Vec<SyntaxError>,
    trivia: Vec<trivia::TriviaPiece>,
    eof_trivia: trivia::TriviaSpan,
}

/// Parses Scala source. Never fails: errors are recorded in the tree.
///
/// Panics if `text` is 4 GiB or larger, since offsets are `u32`.
pub fn parse(text: &str) -> Parse {
    assert!(
        TextSize::try_from(text.len()).is_ok(),
        "source must be under 4 GiB"
    );
    let mut parser = Parser::new();
    parser
        .set_language(language())
        .expect("tree-sitter-scala is compatible with the tree-sitter library");
    // parse only returns None when cancelled or timed out; we set neither.
    let ts_tree = parser.parse(text, None).expect("parse was not cancelled");
    build(text.to_string(), &ts_tree)
}

impl Parse {
    /// The root node.
    pub fn syntax(&self) -> SyntaxNode<'_> {
        SyntaxNode { tree: self, id: 0 }
    }

    /// Every `ERROR` node and missing token, in source order.
    pub fn errors(&self) -> &[SyntaxError] {
        &self.errors
    }

    /// 1-based line and column (in characters) of an offset.
    pub fn line_col(&self, offset: TextSize) -> (usize, usize) {
        let offset = usize::from(offset).min(self.text.len());
        let before = &self.text[..offset];
        let line_start = before.rfind('\n').map_or(0, |i| i + 1);
        let line = before.matches('\n').count() + 1;
        let col = before[line_start..].chars().count() + 1;
        (line, col)
    }
}

/// A syntax error found while parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    message: String,
    range: TextRange,
}

impl SyntaxError {
    pub fn new(message: impl Into<String>, range: TextRange) -> Self {
        SyntaxError {
            message: message.into(),
            range,
        }
    }

    pub fn range(&self) -> TextRange {
        self.range
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SyntaxError {}

fn collect_errors(root: SyntaxNode<'_>) -> Vec<SyntaxError> {
    let mut errors = Vec::new();
    for element in root.descendants_with_tokens() {
        match element {
            NodeOrToken::Node(node) if node.kind() == SyntaxKind::ERROR => {
                errors.push(SyntaxError::new(
                    format!("unexpected {}", describe(node.text())),
                    node.text_range(),
                ))
            }
            NodeOrToken::Token(token) if token.is_missing() => errors.push(SyntaxError::new(
                format!("missing {:?}", token.kind()),
                token.text_range(),
            )),
            _ => {}
        }
    }
    errors
}

fn describe(text: &str) -> String {
    let first_line = text.trim().lines().next().unwrap_or("");
    if first_line.is_empty() {
        return "input".to_string();
    }
    let short: String = first_line.chars().take(40).collect();
    if short.len() < first_line.len() {
        format!("`{short}...`")
    } else {
        format!("`{short}`")
    }
}

// ---------------------------------------------------------------------------
// Handles.

/// Either a node or a token.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum NodeOrToken<N, T> {
    Node(N),
    Token(T),
}

impl<N, T> NodeOrToken<N, T> {
    pub fn into_node(self) -> Option<N> {
        match self {
            NodeOrToken::Node(n) => Some(n),
            NodeOrToken::Token(_) => None,
        }
    }

    pub fn into_token(self) -> Option<T> {
        match self {
            NodeOrToken::Token(t) => Some(t),
            NodeOrToken::Node(_) => None,
        }
    }

    pub fn as_node(&self) -> Option<&N> {
        match self {
            NodeOrToken::Node(n) => Some(n),
            NodeOrToken::Token(_) => None,
        }
    }

    pub fn as_token(&self) -> Option<&T> {
        match self {
            NodeOrToken::Token(t) => Some(t),
            NodeOrToken::Node(_) => None,
        }
    }
}

pub type SyntaxElement<'t> = NodeOrToken<SyntaxNode<'t>, SyntaxToken<'t>>;

/// An interior node of the tree. Cheap to copy; borrows the [`Parse`].
#[derive(Clone, Copy)]
pub struct SyntaxNode<'t> {
    tree: &'t Parse,
    id: NodeId,
}

impl<'t> SyntaxNode<'t> {
    fn data(&self) -> &'t NodeData {
        &self.tree.nodes[self.id]
    }

    fn element_at(&self, index: usize) -> Option<SyntaxElement<'t>> {
        let child = self.data().children.get(index)?;
        Some(match child {
            Child::Node(id) => NodeOrToken::Node(SyntaxNode {
                tree: self.tree,
                id: *id,
            }),
            Child::Token(_) => NodeOrToken::Token(SyntaxToken {
                tree: self.tree,
                parent: self.id,
                index,
            }),
        })
    }

    pub fn kind(&self) -> SyntaxKind {
        self.data().kind
    }

    pub fn text_range(&self) -> TextRange {
        self.data().range
    }

    pub fn text(&self) -> &'t str {
        &self.tree.text[self.text_range()]
    }

    /// The tree-sitter field name this node has in its parent, such as
    /// `name` or `body`.
    pub fn field_name(&self) -> Option<&'static str> {
        self.data().field_name
    }

    /// Whether this is a comment or inside one. tree-sitter calls comments
    /// "extras" because they can appear anywhere.
    pub fn is_extra(&self) -> bool {
        self.data().extra
    }

    pub fn parent(&self) -> Option<SyntaxNode<'t>> {
        self.data().parent.map(|id| SyntaxNode {
            tree: self.tree,
            id,
        })
    }

    /// Position among the parent's children (nodes and tokens).
    pub fn index(&self) -> usize {
        self.data().index
    }

    /// Child nodes, without tokens.
    pub fn children(&self) -> impl Iterator<Item = SyntaxNode<'t>> + use<'t> {
        self.children_with_tokens()
            .filter_map(NodeOrToken::into_node)
    }

    /// All direct children, nodes and tokens, in source order.
    pub fn children_with_tokens(&self) -> impl Iterator<Item = SyntaxElement<'t>> + use<'t> {
        let node = *self;
        (0..self.data().children.len()).filter_map(move |i| node.element_at(i))
    }

    pub fn first_child_or_token(&self) -> Option<SyntaxElement<'t>> {
        self.element_at(0)
    }

    pub fn last_child_or_token(&self) -> Option<SyntaxElement<'t>> {
        self.element_at(self.data().children.len().checked_sub(1)?)
    }

    pub fn next_sibling_or_token(&self) -> Option<SyntaxElement<'t>> {
        self.parent()?.element_at(self.index() + 1)
    }

    pub fn prev_sibling_or_token(&self) -> Option<SyntaxElement<'t>> {
        self.parent()?.element_at(self.index().checked_sub(1)?)
    }

    /// The first child (node or token) with the given tree-sitter field name.
    pub fn child_by_field_name(&self, field_name: &str) -> Option<SyntaxElement<'t>> {
        self.children_with_tokens()
            .find(|child| child.field_name() == Some(field_name))
    }

    /// Enter and leave events for this node and all nodes below it.
    pub fn preorder(&self) -> impl Iterator<Item = WalkEvent<SyntaxNode<'t>>> + use<'t> {
        self.preorder_with_tokens().filter_map(|event| match event {
            WalkEvent::Enter(NodeOrToken::Node(n)) => Some(WalkEvent::Enter(n)),
            WalkEvent::Leave(NodeOrToken::Node(n)) => Some(WalkEvent::Leave(n)),
            _ => None,
        })
    }

    /// Enter and leave events for this node and everything below it.
    pub fn preorder_with_tokens(&self) -> PreorderWithTokens<'t> {
        PreorderWithTokens {
            start: *self,
            next: Some(WalkEvent::Enter(NodeOrToken::Node(*self))),
        }
    }

    /// This node and all nodes below it, in source order.
    pub fn descendants(&self) -> impl Iterator<Item = SyntaxNode<'t>> + use<'t> {
        self.preorder().filter_map(WalkEvent::into_enter)
    }

    /// This node and everything below it, in source order.
    pub fn descendants_with_tokens(&self) -> impl Iterator<Item = SyntaxElement<'t>> + use<'t> {
        self.preorder_with_tokens()
            .filter_map(WalkEvent::into_enter)
    }

    /// All tokens under this node, in source order. Their texts concatenate
    /// to `self.text()`.
    pub fn tokens(&self) -> impl Iterator<Item = SyntaxToken<'t>> + use<'t> {
        self.descendants_with_tokens()
            .filter_map(NodeOrToken::into_token)
    }
}

impl PartialEq for SyntaxNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.tree, other.tree) && self.id == other.id
    }
}

impl Eq for SyntaxNode<'_> {}

impl Hash for SyntaxNode<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.tree, state);
        self.id.hash(state);
    }
}

/// `{:?}` prints `kind@range`; `{:#?}` prints the whole subtree, one element
/// per line.
impl fmt::Debug for SyntaxNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !f.alternate() {
            return write!(f, "{:?}@{:?}", self.kind(), self.text_range());
        }
        let mut depth = 0;
        for event in self.preorder_with_tokens() {
            let element = match event {
                WalkEvent::Enter(element) => element,
                WalkEvent::Leave(element) => {
                    if element.as_node().is_some() {
                        depth -= 1;
                    }
                    continue;
                }
            };
            write!(f, "{:width$}", "", width = depth * 2)?;
            if let Some(field_name) = element.field_name() {
                write!(f, "{field_name}: ")?;
            }
            match element {
                NodeOrToken::Node(node) => {
                    writeln!(f, "{node:?}")?;
                    depth += 1;
                }
                NodeOrToken::Token(token) => writeln!(f, "{token:?}")?,
            }
        }
        Ok(())
    }
}

/// Prints the node's source text.
impl fmt::Display for SyntaxNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}

/// A leaf of the tree: a keyword, identifier, literal, punctuation, comment
/// or whitespace.
#[derive(Clone, Copy)]
pub struct SyntaxToken<'t> {
    tree: &'t Parse,
    parent: NodeId,
    index: usize,
}

impl<'t> SyntaxToken<'t> {
    fn data(&self) -> &'t TokenData {
        match &self.tree.nodes[self.parent].children[self.index] {
            Child::Token(data) => data,
            Child::Node(_) => unreachable!("token handles point at tokens"),
        }
    }

    fn parent_node(&self) -> SyntaxNode<'t> {
        SyntaxNode {
            tree: self.tree,
            id: self.parent,
        }
    }

    pub fn kind(&self) -> SyntaxKind {
        self.data().kind
    }

    pub fn text_range(&self) -> TextRange {
        self.data().range
    }

    pub fn text(&self) -> &'t str {
        &self.tree.text[self.text_range()]
    }

    pub fn field_name(&self) -> Option<&'static str> {
        self.data().field_name
    }

    /// Always `Some`.
    pub fn parent(&self) -> Option<SyntaxNode<'t>> {
        Some(self.parent_node())
    }

    /// Position among the parent's children (nodes and tokens).
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn next_sibling_or_token(&self) -> Option<SyntaxElement<'t>> {
        self.parent_node().element_at(self.index + 1)
    }

    pub fn prev_sibling_or_token(&self) -> Option<SyntaxElement<'t>> {
        self.parent_node().element_at(self.index.checked_sub(1)?)
    }

    /// A zero-width token tree-sitter inserted to recover from an error.
    pub fn is_missing(&self) -> bool {
        self.data().missing
    }

    /// Whether this token is part of a comment.
    pub fn is_extra(&self) -> bool {
        self.data().extra
    }

    /// Whitespace and comments.
    pub fn is_trivia(&self) -> bool {
        self.kind() == SyntaxKind::WHITESPACE || self.is_extra()
    }
}

impl PartialEq for SyntaxToken<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.tree, other.tree)
            && self.parent == other.parent
            && self.index == other.index
    }
}

impl Eq for SyntaxToken<'_> {}

impl Hash for SyntaxToken<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.tree, state);
        self.parent.hash(state);
        self.index.hash(state);
    }
}

impl fmt::Debug for SyntaxToken<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_missing() {
            f.write_str("MISSING ")?;
        }
        write!(f, "{:?}@{:?}", self.kind(), self.text_range())?;
        // Anonymous tokens are their own text, so only show it for named ones.
        if self.kind().is_named() {
            write!(f, " {:?}", self.text())?;
        }
        Ok(())
    }
}

/// Prints the token's source text.
impl fmt::Display for SyntaxToken<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text())
    }
}

impl<'t> SyntaxElement<'t> {
    pub fn kind(&self) -> SyntaxKind {
        match self {
            NodeOrToken::Node(n) => n.kind(),
            NodeOrToken::Token(t) => t.kind(),
        }
    }

    pub fn text_range(&self) -> TextRange {
        match self {
            NodeOrToken::Node(n) => n.text_range(),
            NodeOrToken::Token(t) => t.text_range(),
        }
    }

    pub fn text(&self) -> &'t str {
        match self {
            NodeOrToken::Node(n) => n.text(),
            NodeOrToken::Token(t) => t.text(),
        }
    }

    pub fn field_name(&self) -> Option<&'static str> {
        match self {
            NodeOrToken::Node(n) => n.field_name(),
            NodeOrToken::Token(t) => t.field_name(),
        }
    }

    pub fn parent(&self) -> Option<SyntaxNode<'t>> {
        match self {
            NodeOrToken::Node(n) => n.parent(),
            NodeOrToken::Token(t) => t.parent(),
        }
    }

    pub fn index(&self) -> usize {
        match self {
            NodeOrToken::Node(n) => n.index(),
            NodeOrToken::Token(t) => t.index(),
        }
    }

    pub fn next_sibling_or_token(&self) -> Option<SyntaxElement<'t>> {
        match self {
            NodeOrToken::Node(n) => n.next_sibling_or_token(),
            NodeOrToken::Token(t) => t.next_sibling_or_token(),
        }
    }

    pub fn prev_sibling_or_token(&self) -> Option<SyntaxElement<'t>> {
        match self {
            NodeOrToken::Node(n) => n.prev_sibling_or_token(),
            NodeOrToken::Token(t) => t.prev_sibling_or_token(),
        }
    }
}

// ---------------------------------------------------------------------------
// Traversal.

/// A step in a tree walk: every element is entered, then left after
/// everything below it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum WalkEvent<T> {
    Enter(T),
    Leave(T),
}

impl<T> WalkEvent<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> WalkEvent<U> {
        match self {
            WalkEvent::Enter(x) => WalkEvent::Enter(f(x)),
            WalkEvent::Leave(x) => WalkEvent::Leave(f(x)),
        }
    }

    pub fn into_enter(self) -> Option<T> {
        match self {
            WalkEvent::Enter(x) => Some(x),
            WalkEvent::Leave(_) => None,
        }
    }
}

/// Depth-first, source-order walk. Moves between siblings and parents
/// without recursion, so deeply nested code can't overflow the stack.
pub struct PreorderWithTokens<'t> {
    start: SyntaxNode<'t>,
    next: Option<WalkEvent<SyntaxElement<'t>>>,
}

impl<'t> Iterator for PreorderWithTokens<'t> {
    type Item = WalkEvent<SyntaxElement<'t>>;

    fn next(&mut self) -> Option<Self::Item> {
        let event = self.next.take()?;
        self.next = match event {
            WalkEvent::Enter(NodeOrToken::Node(node)) => Some(match node.first_child_or_token() {
                Some(child) => WalkEvent::Enter(child),
                None => WalkEvent::Leave(NodeOrToken::Node(node)),
            }),
            WalkEvent::Enter(token) => Some(WalkEvent::Leave(token)),
            WalkEvent::Leave(element) => {
                if element == NodeOrToken::Node(self.start) {
                    None
                } else {
                    Some(match element.next_sibling_or_token() {
                        Some(sibling) => WalkEvent::Enter(sibling),
                        None => WalkEvent::Leave(NodeOrToken::Node(
                            element.parent().expect("non-root elements have a parent"),
                        )),
                    })
                }
            }
        };
        Some(event)
    }
}

// ---------------------------------------------------------------------------
// Building the tree from tree-sitter's output.

struct OpenNode {
    id: NodeId,
    /// End of the last child added so far.
    pos: usize,
    end: usize,
}

struct TreeBuilder<'s> {
    text: &'s str,
    nodes: Vec<NodeData>,
    open: Vec<OpenNode>,
}

fn size(n: usize) -> TextSize {
    TextSize::try_from(n).expect("parse() checks the source is under 4 GiB")
}

impl TreeBuilder<'_> {
    /// Clamps a tree-sitter range so it nests inside the open node and never
    /// goes backwards, then fills any gap before it.
    fn place(&mut self, start: usize, end: usize) -> (usize, usize) {
        let open = self.open.last().expect("a node is open");
        let start = start.clamp(open.pos, open.end);
        let end = end.clamp(start, open.end);
        let (id, gap_start) = (open.id, open.pos);
        self.fill_gap(id, gap_start, start);
        self.open.last_mut().expect("a node is open").pos = end;
        (start, end)
    }

    fn start_node(
        &mut self,
        kind: SyntaxKind,
        field_name: Option<&'static str>,
        start: usize,
        end: usize,
        extra: bool,
    ) {
        let parent = self.open.last().map(|open| open.id);
        let (start, end) = if parent.is_some() {
            self.place(start, end)
        } else {
            (start, end)
        };
        let id = self.nodes.len();
        let index = match parent {
            Some(parent) => {
                let children = &mut self.nodes[parent].children;
                children.push(Child::Node(id));
                children.len() - 1
            }
            None => 0,
        };
        let extra = extra || parent.is_some_and(|p| self.nodes[p].extra);
        self.nodes.push(NodeData {
            kind,
            field_name,
            range: TextRange::new(size(start), size(end)),
            parent,
            index,
            extra,
            children: Vec::new(),
        });
        self.open.push(OpenNode {
            id,
            pos: start,
            end,
        });
    }

    fn token(
        &mut self,
        kind: SyntaxKind,
        field_name: Option<&'static str>,
        start: usize,
        end: usize,
        extra: bool,
        missing: bool,
    ) {
        let (start, end) = self.place(start, end);
        let parent = self.open.last().expect("a node is open").id;
        let extra = extra || self.nodes[parent].extra;
        self.push_token(parent, kind, field_name, start, end, extra, missing);
    }

    fn finish_node(&mut self) {
        let open = self.open.pop().expect("a node is open");
        self.fill_gap(open.id, open.pos, open.end);
    }

    #[allow(clippy::too_many_arguments)]
    fn push_token(
        &mut self,
        parent: NodeId,
        kind: SyntaxKind,
        field_name: Option<&'static str>,
        start: usize,
        end: usize,
        extra: bool,
        missing: bool,
    ) {
        self.nodes[parent].children.push(Child::Token(TokenData {
            kind,
            field_name,
            range: TextRange::new(size(start), size(end)),
            extra,
            missing,
            leading: Default::default(),
            trailing: Default::default(),
        }));
    }

    /// Covers `start..end` inside `parent` with a token for text tree-sitter
    /// didn't assign to any child.
    fn fill_gap(&mut self, parent: NodeId, start: usize, end: usize) {
        if start >= end {
            return;
        }
        let text = &self.text[start..end];
        let node = &self.nodes[parent];
        let kind = if text.chars().all(char::is_whitespace) && !is_literal(node.kind) {
            SyntaxKind::WHITESPACE
        } else {
            SyntaxKind::TEXT
        };
        let extra = node.extra;
        self.push_token(parent, kind, None, start, end, extra, false);
    }
}

/// Nodes whose uncovered text is content, not whitespace between tokens.
fn is_literal(kind: SyntaxKind) -> bool {
    kind.name().contains("string") || kind.name().contains("comment")
}

fn field_name(id: std::num::NonZeroU16) -> Option<&'static str> {
    language().field_name_for_id(id.get())
}

fn build(text: String, ts_tree: &tree_sitter::Tree) -> Parse {
    let ts_root = ts_tree.root_node();
    let mut b = TreeBuilder {
        text: &text,
        nodes: Vec::new(),
        open: Vec::new(),
    };

    // Our root always spans the whole file, even when tree-sitter's root
    // starts after leading whitespace.
    b.start_node(SyntaxKind(ts_root.kind_id()), None, 0, text.len(), false);

    let mut cursor = ts_tree.walk();
    if cursor.goto_first_child() {
        'walk: loop {
            let node = cursor.node();
            let kind = SyntaxKind(node.kind_id());
            let field = cursor.field_id().and_then(field_name);
            let (start, end) = (node.start_byte(), node.end_byte());
            // tree-sitter marks comments as "extra", but also some ERROR
            // nodes from error recovery. Those hold code, not comments.
            let extra = node.is_extra() && kind != SyntaxKind::ERROR;

            if node.child_count() == 0 && kind != SyntaxKind::ERROR {
                b.token(kind, field, start, end, extra, node.is_missing());
            } else {
                b.start_node(kind, field, start, end, extra);
                if cursor.goto_first_child() {
                    continue 'walk;
                }
                // An ERROR node with no children: its text becomes one token.
                b.finish_node();
            }

            // Move to the next sibling, finishing parents on the way up.
            loop {
                if cursor.goto_next_sibling() {
                    continue 'walk;
                }
                if !cursor.goto_parent() {
                    break 'walk;
                }
                b.finish_node();
            }
        }
    }
    // Finish the root (when it had no children, or after the walk ends there).
    while !b.open.is_empty() {
        b.finish_node();
    }

    let nodes = b.nodes;
    let mut parse = Parse {
        text,
        nodes,
        errors: Vec::new(),
        trivia: Vec::new(),
        eof_trivia: Default::default(),
    };
    trivia::attach(&mut parse);
    if ts_root.has_error() {
        parse.errors = collect_errors(parse.syntax());
    }
    parse
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The core guarantee: tokens cover the source exactly, in order, and
    /// every node's range matches its tokens.
    fn assert_lossless(source: &str) -> Parse {
        let parse = parse(source);
        let root = parse.syntax();
        let mut pos = TextSize::from(0);
        for token in root.tokens() {
            assert_eq!(
                token.text_range().start(),
                pos,
                "gap or overlap before {token:?}"
            );
            pos = token.text_range().end();
        }
        assert_eq!(usize::from(pos), source.len(), "tokens stop before the end");
        assert_eq!(root.to_string(), source);
        assert_eq!(trivia::reassemble(&parse), source, "trivia doesn't add up");
        for node in root.descendants() {
            let text: String = node.tokens().map(|t| t.text()).collect();
            assert_eq!(text, node.text(), "tokens of {node:?} don't match its text");
        }
        parse
    }

    #[test]
    fn debug_tree_shows_tokens_fields_and_whitespace() {
        let parse = assert_lossless("class Foo\n");
        assert_eq!(
            format!("{:#?}", parse.syntax()),
            concat!(
                "compilation_unit@0..10\n",
                "  class_definition@0..9\n",
                "    \"class\"@0..5\n",
                "    whitespace@5..6 \" \"\n",
                "    name: identifier@6..9 \"Foo\"\n",
                "  whitespace@9..10 \"\\n\"\n",
            )
        );
        assert!(parse.errors().is_empty());
    }

    #[test]
    fn scala2_file_round_trips() {
        assert_lossless(
            r#"package com.example

import scala.collection.mutable.{Map => MMap, _}

/** Docs. */
sealed trait Shape { def area: Double }

// Shapes
case class Circle(r: Double) extends Shape {
  override def area: Double = math.Pi * r * r /* inline */
}

object Main extends App {
  val shapes: List[Shape] = List(Circle(1.0), Circle(2.5))
  for (s <- shapes if s.area > 1) println(s"area: ${s.area}%.2f and $s")
  shapes match {
    case Circle(r) :: _ => println(r)
    case Nil => ()
  }
}
"#,
        );
    }

    #[test]
    fn scala3_file_round_trips() {
        let parse = assert_lossless(
            "enum Color:\n  case Red, Green\n\ndef show(c: Color): String =\n  c match\n    case Color.Red => \"red\"\n    case _ => \"other\"\n\nextension (s: String)\n  def shout: String = s.toUpperCase + \"!\"\n",
        );
        assert!(parse.errors().is_empty(), "{:#?}", parse.syntax());
    }

    #[test]
    fn interpolated_string_text_is_not_whitespace() {
        let parse = assert_lossless(r#"val x = s"${a} ${b}""#);
        let root = parse.syntax();
        assert!(
            !root.tokens().any(|t| t.kind() == SyntaxKind::WHITESPACE
                && t.parent().unwrap().kind().name().contains("string")),
            "{root:#?}"
        );
    }

    #[test]
    fn comments_are_trivia() {
        let parse = assert_lossless("// head\nval x = 1 /* mid */\n// tail");
        let root = parse.syntax();
        let comment_text: String = root
            .tokens()
            .filter(|t| t.is_trivia() && t.kind() != SyntaxKind::WHITESPACE)
            .map(|t| t.text())
            .collect();
        assert_eq!(comment_text, "// head/* mid */// tail", "{root:#?}");
        let code: String = root
            .tokens()
            .filter(|t| !t.is_trivia())
            .map(|t| t.text())
            .collect();
        assert_eq!(code, "valx=1");
    }

    #[test]
    fn child_by_field_name_finds_names() {
        let parse = assert_lossless("object Main { def run(x: Int): Unit = () }");
        let def = parse
            .syntax()
            .descendants()
            .find(|n| n.kind() == "function_definition")
            .unwrap();
        assert_eq!(def.child_by_field_name("name").unwrap().text(), "run");
        assert_eq!(
            def.child_by_field_name("return_type").unwrap().text(),
            "Unit"
        );
    }

    #[test]
    fn children_are_nodes_and_children_with_tokens_are_everything() {
        let parse = parse("val x = 1");
        let val = parse.syntax().children().next().unwrap();
        assert_eq!(val.kind(), "val_definition");
        let nodes: Vec<_> = val.children().collect();
        assert!(nodes.is_empty(), "every child of `val x = 1` is a token");
        let all: Vec<_> = val.children_with_tokens().map(|e| e.text()).collect();
        assert_eq!(all, ["val", " ", "x", " ", "=", " ", "1"]);
    }

    #[test]
    fn siblings_and_index_agree() {
        let parse = parse("val x = 1");
        let val = parse.syntax().first_child_or_token().unwrap();
        let children: Vec<_> = val.as_node().unwrap().children_with_tokens().collect();
        for (i, child) in children.iter().enumerate() {
            assert_eq!(child.index(), i);
            assert_eq!(child.parent(), val.as_node().copied());
            assert_eq!(child.next_sibling_or_token(), children.get(i + 1).copied());
            let prev = i.checked_sub(1).map(|p| children[p]);
            assert_eq!(child.prev_sibling_or_token(), prev);
        }
        assert_eq!(
            val.as_node().unwrap().last_child_or_token(),
            children.last().copied()
        );
    }

    #[test]
    fn preorder_enters_and_leaves_in_order() {
        let parse = parse("class A");
        let events: Vec<String> = parse
            .syntax()
            .preorder()
            .map(|e| match e {
                WalkEvent::Enter(n) => format!("+{}", n.kind()),
                WalkEvent::Leave(n) => format!("-{}", n.kind()),
            })
            .collect();
        assert_eq!(
            events,
            [
                "+compilation_unit",
                "+class_definition",
                "-class_definition",
                "-compilation_unit"
            ]
        );
    }

    #[test]
    fn walking_a_subtree_stays_inside_it() {
        let parse = parse("class A\nclass B");
        let a = parse.syntax().children().next().unwrap();
        let texts: Vec<_> = a.descendants_with_tokens().map(|e| e.text()).collect();
        assert_eq!(texts, ["class A", "class", " ", "A"]);
    }

    #[test]
    fn errors_are_reported_and_tree_stays_lossless() {
        let parse = assert_lossless("object A {\n  def f( = 1\n");
        let errors = parse.errors();
        assert!(!errors.is_empty(), "{:#?}", parse.syntax());
        let (line, _) = parse.line_col(errors[0].range().start());
        assert_eq!(line, 2, "{errors:?}");
        assert!(
            parse
                .syntax()
                .descendants()
                .any(|n| n.kind() == SyntaxKind::ERROR),
            "tree-sitter's error symbol maps to SyntaxKind::ERROR"
        );
    }

    #[test]
    fn edge_cases_round_trip() {
        assert_lossless("");
        assert_lossless("   \n\t\n");
        assert_lossless("val ünïcödé = \"日本語\" // 🎉\n");
        assert_lossless("}}}{{{ ??? ::: @@@ \"unterminated");
        assert_lossless("\r\nobject Win {\r\n  val x = 1\r\n}\r\n");
    }

    #[test]
    fn deep_nesting_does_not_overflow() {
        let depth = 2000;
        let source = format!("val x = {}1{}", "(".repeat(depth), ")".repeat(depth));
        let parse = assert_lossless(&source);
        assert!(format!("{:#?}", parse.syntax()).lines().count() > depth);
    }

    #[test]
    fn iterators_outlive_the_handle_they_came_from() {
        let parse = parse("class A { val x = 1 }");
        // Each handle here is a temporary; the iterators only borrow `parse`.
        let kinds: Vec<_> = parse
            .syntax()
            .children()
            .next()
            .unwrap()
            .children()
            .map(|n| n.kind())
            .collect();
        let first = parse.syntax().first_token().unwrap();
        let mut trivia = first.next_token().unwrap().leading_trivia();
        assert!(trivia.next().is_none());
        assert_eq!(kinds.len(), 1);
    }

    #[test]
    fn our_kinds_dont_collide_with_grammar_kinds() {
        assert!(language().node_kind_count() < usize::from(SyntaxKind::TEXT.0));
    }

    #[test]
    fn line_col_counts_characters() {
        let parse = parse("val a = 1\nval é = 2");
        let text = parse.syntax().text();
        assert_eq!(parse.line_col(0.into()), (1, 1));
        assert_eq!(parse.line_col(size(text.find('=').unwrap())), (1, 7));
        assert_eq!(parse.line_col(size(text.rfind('=').unwrap())), (2, 7));
    }
}

/// Checks every `.scala` file under `$OUF_CORPUS`. Run with:
/// `OUF_CORPUS=path/to/scala/code cargo test --release corpus -- --ignored --nocapture`
#[cfg(test)]
mod corpus {
    use super::*;
    use std::path::{Path, PathBuf};

    fn scala_files(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                scala_files(&path, out);
            } else if path.extension().is_some_and(|e| e == "scala") {
                out.push(path);
            }
        }
    }

    #[test]
    #[ignore = "needs OUF_CORPUS"]
    fn corpus_round_trips() {
        let root = std::env::var("OUF_CORPUS").expect("set OUF_CORPUS to a directory");
        let mut files = Vec::new();
        scala_files(Path::new(&root), &mut files);
        let (mut with_errors, mut comments) = (0, 0);
        for path in &files {
            let Ok(source) = std::fs::read_to_string(path) else {
                continue;
            };
            let parse = parse(&source);
            let root = parse.syntax();
            assert_eq!(root.to_string(), source, "{}", path.display());
            assert_eq!(trivia::reassemble(&parse), source, "{}", path.display());
            for node in root.descendants() {
                let text: String = node.tokens().map(|t| t.text()).collect();
                assert_eq!(text, node.text(), "{node:?} in {}", path.display());
            }
            let attached: usize = root
                .tokens()
                .map(|t| {
                    t.leading_trivia()
                        .chain(t.trailing_trivia())
                        .filter(|t| t.is_comment())
                        .count()
                })
                .sum::<usize>()
                + parse.eof_trivia().filter(|t| t.is_comment()).count();
            let in_tree = root
                .descendants_with_tokens()
                .filter(|e| match e {
                    NodeOrToken::Node(n) => {
                        n.is_extra() && !n.parent().is_some_and(|p| p.is_extra())
                    }
                    NodeOrToken::Token(t) => t.is_extra() && !t.parent().unwrap().is_extra(),
                })
                .count();
            assert_eq!(
                attached,
                in_tree,
                "every comment attached once: {}",
                path.display()
            );
            comments += attached;
            with_errors += usize::from(!parse.errors().is_empty());
        }
        println!(
            "{} files, {comments} comments, {with_errors} files with syntax errors",
            files.len()
        );
    }
}
