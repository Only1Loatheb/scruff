use std::fmt;

mod indent;
mod spacing;

use crate::doc::{Doc, PrintOptions, print};
use crate::lossless_tree::{
    Difference, NodeOrToken, Parse, SyntaxElement, SyntaxError, SyntaxKind, SyntaxNode,
    SyntaxToken, SyntaxTrivia, TextRange, TextSize, TriviaKind, VerbatimRange, VerbatimReason,
    check_equivalent, parse,
};

/// Formatting is deliberately not configurable beyond this. Indentation is
/// always two spaces, and lines end in `\n`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatOptions {
    /// Defaults to 100, sbt's scalafmt `maxColumn`.
    pub max_width: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions { max_width: 100 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatError {
    /// The input doesn't parse cleanly.
    SyntaxErrors(Vec<SyntaxError>),
    /// The formatter produced code that differs from the input. A bug in a
    /// formatting rule; the input is left alone.
    NotEquivalent(Difference),
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::SyntaxErrors(errors) => {
                write!(f, "not formatting code with syntax errors")?;
                if let Some(first) = errors.first() {
                    write!(f, ", the first: {first}")?;
                }
                Ok(())
            }
            FormatError::NotEquivalent(difference) => {
                write!(f, "formatting would change the code: {difference}")
            }
        }
    }
}

impl std::error::Error for FormatError {}

/// Formats Scala source.
pub fn format(source: &str, options: &FormatOptions) -> Result<String, FormatError> {
    format_with(source, options, Rules::Structural, STYLE)
}

/// Which rules build the document.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Rules {
    /// A node's own rule where it has one, otherwise the fallback.
    Structural,
    /// Only the fallback: children in order, spacing from [`spacing`].
    /// Tests compare it with [`Rules::Structural`].
    #[cfg_attr(not(test), allow(dead_code))]
    Fallback,
}

/// Choices scuff makes for everyone, gathered in one place so they're easy
/// to revisit. They aren't options: `format` always uses [`STYLE`], and
/// tests may try others.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Style {
    /// Blank lines in a row, at most, between definitions in a template:
    /// the body of a class, object, trait or enum, an extension's methods,
    /// a package, or the file.
    max_blank_lines_in_templates: usize,
    /// Blank lines in a row, at most, anywhere else: in a block or a method
    /// body, including between nested functions.
    max_blank_lines_in_blocks: usize,
}

/// After Black, the Python formatter, which allows 2 blank lines between
/// module-level definitions and 1 inside functions. Scala's templates play
/// the part of Python's modules. sbt's code agrees: its runs of 2 in blocks
/// are all in comments or `format: off` blocks, which are copied as
/// written.
const STYLE: Style = Style {
    max_blank_lines_in_templates: 2,
    max_blank_lines_in_blocks: 1,
};

fn format_with(
    source: &str,
    options: &FormatOptions,
    rules: Rules,
    style: Style,
) -> Result<String, FormatError> {
    let parse = parse(source);
    if !parse.errors().is_empty() {
        return Err(FormatError::SyntaxErrors(parse.errors().to_vec()));
    }
    let print_options = PrintOptions {
        max_width: options.max_width,
        indent: "  ".to_string(),
        line_ending: "\n".to_string(),
    };
    let reindent = indent::Reindent::new(&parse);
    let reindented = !reindent.is_empty();
    let formatted = print(&build_doc(&parse, rules, style, reindent), &print_options);
    match check_equivalent(source, &formatted) {
        Ok(()) => Ok(formatted),
        // Where tree-sitter-scala reads the layout differently from scalac,
        // moving lines can change the tree. Keep the indentation as written
        // then, and the rest of the formatting.
        Err(_) if reindented => {
            let doc = build_doc(&parse, rules, style, indent::Reindent::default());
            let formatted = print(&doc, &print_options);
            check_equivalent(source, &formatted).map_err(FormatError::NotEquivalent)?;
            Ok(formatted)
        }
        Err(difference) => Err(FormatError::NotEquivalent(difference)),
    }
}

/// The document for a parsed file, before printing.
pub fn to_doc<'t>(parse: &'t Parse) -> Doc<'t> {
    build_doc(
        parse,
        Rules::Structural,
        STYLE,
        indent::Reindent::new(parse),
    )
}

fn build_doc<'t>(
    parse: &'t Parse,
    rules: Rules,
    style: Style,
    reindent: indent::Reindent,
) -> Doc<'t> {
    let mut formatter = Formatter {
        parse,
        rules,
        text: parse.syntax().text(),
        verbatim: parse.verbatim_ranges(),
        next_verbatim: 0,
        copied_until: TextSize::from(0),
        leading_done: None,
        trailing_skipped: Vec::new(),
        reindent,
        at_file_start: true,
        line_breaks: 0,
        style,
    };
    let body = formatter.node(parse.syntax());
    // The file ends with its last code or comment and one line break, as in
    // scalafmt, unless what's at the end is copied verbatim.
    let mut eof: Vec<_> = parse.eof_trivia().collect();
    while eof.last().is_some_and(|t| {
        matches!(t.kind(), TriviaKind::Newline | TriviaKind::Whitespace)
            && formatter.is_free(t.text_range().start())
    }) {
        eof.pop();
    }
    // Popping stopped at verbatim whitespace: its line breaks are kept.
    let ends_verbatim = eof
        .last()
        .is_some_and(|t| matches!(t.kind(), TriviaKind::Newline | TriviaKind::Whitespace));
    let mut docs = Vec::new();
    formatter.trivia_within_lines(&mut docs, &eof, Gap::LineStart, None);
    let end = if formatter.at_file_start || ends_verbatim {
        Doc::Nil
    } else {
        Doc::hardline()
    };
    body + Doc::concat(docs) + end
}

/// Builds a [`Doc`] from the tree, copying verbatim ranges as they're
/// reached.
struct Formatter<'t> {
    parse: &'t Parse,
    rules: Rules,
    text: &'t str,
    verbatim: &'t [VerbatimRange],
    /// The first verbatim range that hasn't ended before the current
    /// position.
    next_verbatim: usize,
    /// End of the last verbatim range copied; source before it is done.
    copied_until: TextSize,
    /// Where the token starts whose leading trivia a rule already printed,
    /// ahead of a group that must not contain its line break.
    leading_done: Option<TextSize>,
    /// Where the tokens start whose trailing gap a rule prints itself, such
    /// as the line break before a `}`. Nested rules can each add one.
    trailing_skipped: Vec<TextSize>,
    /// New indentation for lines whose indentation changes.
    reindent: indent::Reindent,
    /// Whether no code or comment has been printed yet: line breaks before
    /// the first one are dropped.
    at_file_start: bool,
    /// Line breaks since the last code or comment.
    line_breaks: usize,
    style: Style,
}

/// What comes just before a piece of trivia on its line.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Gap {
    /// Nothing: the start of a line, or of the file.
    LineStart,
    /// Code, with no space yet.
    AfterCode,
    /// A comment, with no space yet.
    AfterComment,
    /// Whitespace kept as written.
    Space,
}

/// What a stretch of source is, so it can be printed the right way.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Piece {
    Code,
    Whitespace,
    Newline,
    Comment,
}

impl<'t> Formatter<'t> {
    fn node(&mut self, node: SyntaxNode<'t>) -> Doc<'t> {
        if self.rules == Rules::Structural
            && let Some(doc) = self.structural(node)
        {
            return doc;
        }
        self.fallback(node)
    }

    /// The node's own rule, if its kind has one and it applies here.
    fn structural(&mut self, node: SyntaxNode<'t>) -> Option<Doc<'t>> {
        match node.kind() {
            // sbt's style: arguments indent 2 spaces and parameters 4, as
            // in scalafmt's `indent.callSite` and `indent.defnSite`.
            SyntaxKind::Arguments => self.list(node, 1),
            SyntaxKind::Parameters | SyntaxKind::ClassParameters => self.list(node, 2),
            // Type parameters and arguments follow the same layout.
            SyntaxKind::TypeArguments => self.list(node, 1),
            SyntaxKind::TypeParameters => self.list(node, 2),
            SyntaxKind::InfixExpression => self.infix(node),
            SyntaxKind::Block => self.braced(node),
            SyntaxKind::CallExpression
            | SyntaxKind::FieldExpression
            | SyntaxKind::GenericFunction => self.select_chain(node),
            _ => None,
        }
    }

    /// A list in parentheses or brackets, such as arguments, parameters, or
    /// type arguments and parameters: on one line if it fits, otherwise one
    /// item per line, nested `levels` deep, with `)` or `]` on its own line
    /// (scalafmt's `danglingParentheses`).
    ///
    /// Only for a list written on one line with no comments. A list over
    /// several lines keeps its line breaks until indentation is handled.
    fn list(&mut self, node: SyntaxNode<'t>, levels: usize) -> Option<Doc<'t>> {
        if !self.can_break(node) {
            return None;
        }
        let children: Vec<_> = node.non_trivia_children().collect();
        let (NodeOrToken::Token(open), NodeOrToken::Token(close)) =
            (children.first()?, children.last()?)
        else {
            return None;
        };
        if !matches!((open.text(), close.text()), ("(", ")") | ("[", "]")) || children.len() < 3 {
            return None;
        }

        // The items between `(` and `)`, split at commas.
        let mut items = vec![Vec::new()];
        let mut commas = Vec::new();
        for child in &children[1..children.len() - 1] {
            match child {
                NodeOrToken::Token(t) if t.text() == "," => {
                    commas.push(*t);
                    items.push(Vec::new());
                }
                _ => items.last_mut().unwrap().push(*child),
            }
        }
        // Empty items: a trailing comma, which needs `)` on the next line.
        if items.iter().any(Vec::is_empty) {
            return None;
        }
        // The gaps that become line breaks must be ones the spacing rules
        // decide, so the list printed flat matches the fallback.
        let first = std::iter::successors(open.next_token(), SyntaxToken::next_token)
            .find(|t| !t.is_trivia())?;
        if self.rule_between(open, &first) != Some(false) {
            return None;
        }
        for comma in &commas {
            let next = std::iter::successors(comma.next_token(), SyntaxToken::next_token)
                .find(|t| !t.is_trivia())?;
            if self.rule_between(comma, &next) != Some(true) {
                return None;
            }
        }

        // A lambda alone in the arguments keeps its head with the `(` and
        // breaks before its body instead, as in sbt's code: `f(x =>`, the
        // body, then `)`.
        let lambda = match items.as_slice() {
            [item] if levels == 1 => match item.as_slice() {
                [NodeOrToken::Node(node)] if node.kind() == SyntaxKind::LambdaExpression => {
                    self.lambda_parts(*node)
                }
                _ => None,
            },
            _ => None,
        };
        if let Some((head, arrow, body)) = lambda {
            let open = self.token_without_trailing(*open);
            let mut docs: Vec<_> = head.into_iter().map(|e| self.element(e)).collect();
            docs.push(self.token_without_trailing(arrow));
            let body = (Doc::line() + self.element(body)).nest();
            let close_text = self.token_without_trailing(*close);
            let close_trailing = self.trailing(*close);
            return Some(
                open + (Doc::concat(docs) + body + Doc::softline() + close_text)
                    .group()
                    .from_line_indent()
                    + close_trailing,
            );
        }

        let open = self.token_without_trailing(*open);
        let mut inner = vec![Doc::softline()];
        let mut commas = commas.into_iter();
        for (i, item) in items.into_iter().enumerate() {
            if i > 0 {
                let comma = commas.next().unwrap();
                inner.push(self.token_without_trailing(comma));
                inner.push(Doc::line());
            }
            for element in item {
                inner.push(self.element(element));
            }
        }
        let mut inner = Doc::concat(inner);
        for _ in 0..levels {
            inner = inner.nest();
        }
        let close_text = self.token_without_trailing(*close);
        let close_trailing = self.trailing(*close);
        Some(
            open + (inner + Doc::softline() + close_text)
                .group()
                .from_line_indent()
                + close_trailing,
        )
    }

    /// A block in braces holding one expression, such as `g { f(x) }` or
    /// `xs.map { x => f(x) }`: on one line if it fits, otherwise the
    /// expression goes on the next line indented 2 spaces and `}` on its own
    /// line, as in sbt's code. A lambda keeps `{ x =>` on the first line.
    fn braced(&mut self, node: SyntaxNode<'t>) -> Option<Doc<'t>> {
        if !self.can_break(node) {
            return None;
        }
        let children: Vec<_> = node.non_trivia_children().collect();
        let [NodeOrToken::Token(open), inner, NodeOrToken::Token(close)] = children.as_slice()
        else {
            return None;
        };
        let name = inner.kind().name();
        if open.text() != "{"
            || close.text() != "}"
            || name.ends_with("_definition")
            || name.ends_with("_declaration")
        {
            return None;
        }
        // The spaces inside the braces must be ones the spacing rules put
        // there, so the block printed flat matches the fallback.
        let first = std::iter::successors(open.next_token(), SyntaxToken::next_token)
            .find(|t| !t.is_trivia())?;
        let last = std::iter::successors(close.prev_token(), SyntaxToken::prev_token)
            .find(|t| !t.is_trivia())?;
        if self.rule_between(open, &first) != Some(true)
            || self.rule_between(&last, close) != Some(true)
        {
            return None;
        }
        let lambda = match inner {
            NodeOrToken::Node(n) if n.kind() == SyntaxKind::LambdaExpression => {
                Some(self.lambda_parts(*n)?)
            }
            _ => None,
        };

        let open = self.token_without_trailing(*open);
        let mut docs = Vec::new();
        let body = match lambda {
            Some((head, arrow, body)) => {
                docs.push(Doc::text(" "));
                docs.extend(head.into_iter().map(|e| self.element(e)));
                docs.push(self.token_without_trailing(arrow));
                body
            }
            None => *inner,
        };
        self.trailing_skipped.push(last.text_range().start());
        let body = (Doc::line() + self.element(body)).nest();
        let close_text = self.token_without_trailing(*close);
        let close_trailing = self.trailing(*close);
        Some(
            open + (Doc::concat(docs) + body + Doc::line() + close_text)
                .group()
                .from_line_indent()
                + close_trailing,
        )
    }

    /// A select chain with at least two calls, such as `xs.map(f).filter(g)`:
    /// on one line if it fits, otherwise broken before the first call's `.`
    /// and every `.` after it, each indented 2 spaces, as scalafmt's classic
    /// select chains in sbt's code. Field selects before the first call stay
    /// with the head: `sys.env` in `sys.env.get(k).map(f)`.
    fn select_chain(&mut self, node: SyntaxNode<'t>) -> Option<Doc<'t>> {
        // Only the outermost node of a chain; the ones inside are its parts.
        if let Some(parent) = node.parent()
            && chain_spine(parent).is_some_and(|spine| spine == NodeOrToken::Node(node))
        {
            return None;
        }
        if !self.can_break(node) {
            return None;
        }
        // Walk down the spine, from the last select to the head. `pending`
        // holds what follows the current part: arguments and type arguments.
        let mut segments: Vec<(Vec<SyntaxElement<'t>>, bool)> = Vec::new();
        let mut pending: Vec<SyntaxElement<'t>> = Vec::new();
        let mut applied = false;
        let mut current = NodeOrToken::Node(node);
        let head = loop {
            let NodeOrToken::Node(n) = current else {
                break current;
            };
            let Some(spine) = chain_spine(n) else {
                break current;
            };
            let extras: Vec<_> = n.non_trivia_children().filter(|c| *c != spine).collect();
            if n.kind() == SyntaxKind::FieldExpression {
                let mut segment = extras;
                segment.extend(pending.drain(..).rev());
                segments.push((segment, applied));
                applied = false;
            } else {
                applied |= n.kind() == SyntaxKind::CallExpression;
                pending.extend(extras.into_iter().rev());
            }
            current = spine;
        };
        segments.reverse();
        let calls = segments.iter().filter(|(_, applied)| *applied).count();
        let first_call = segments.iter().position(|(_, applied)| *applied)?;
        if calls < 2 {
            return None;
        }

        let leading = self.leading_of(node);
        let mut first_line = vec![self.element(head)];
        first_line.extend(pending.into_iter().rev().map(|e| self.element(e)));
        let mut segments = segments.into_iter().map(|(segment, _)| segment);
        for segment in segments.by_ref().take(first_call) {
            first_line.extend(segment.into_iter().map(|e| self.element(e)));
        }
        let mut rest = Vec::new();
        for segment in segments {
            rest.push(Doc::softline());
            rest.extend(segment.into_iter().map(|e| self.element(e)));
        }
        Some(
            leading
                + (Doc::concat(first_line) + Doc::concat(rest).nest())
                    .group()
                    .from_line_indent(),
        )
    }

    /// A boolean chain, such as `a && b && c`: on one line if it fits,
    /// otherwise broken after each operator, with the operands after the
    /// first indented 2 spaces, as in sbt's code. Only the operators at the
    /// chain's own precedence break; an operand with operators that bind
    /// tighter, like `a == b` in `a == b && c`, is a chain of its own.
    ///
    /// Only `&&` and `||` chains break. With sbt's configuration, scalafmt
    /// never adds breaks inside other infix chains, such as `"org" % "name"
    /// % "1.0"` or string concatenation, and leaves those lines long.
    fn infix(&mut self, node: SyntaxNode<'t>) -> Option<Doc<'t>> {
        if !self.can_break(node) {
            return None;
        }
        let (mut operands, mut operators) = (Vec::new(), Vec::new());
        flatten_infix(node, &mut operands, &mut operators)?;
        for op in &operators {
            if !matches!(op.text(), "&&" | "||") {
                return None;
            }
            let next = std::iter::successors(op.next_token(), SyntaxToken::next_token)
                .find(|t| !t.is_trivia())?;
            if self.rule_between(op, &next) != Some(true) {
                return None;
            }
        }
        let leading = self.leading_of(node);
        let mut operands = operands.into_iter();
        let first = self.element(operands.next()?);
        let mut rest = Vec::new();
        for (op, operand) in operators.into_iter().zip(operands) {
            rest.push(self.token_without_trailing(op));
            rest.push(Doc::line());
            rest.push(self.element(operand));
        }
        Some(
            leading
                + (first + Doc::concat(rest).nest())
                    .group()
                    .from_line_indent(),
        )
    }

    /// Whether a rule may add line breaks inside `node`: it's written on one
    /// line, has no comments inside, and starts a line of code, since broken
    /// lines are indented from that line's indentation. Nodes over several
    /// lines keep their breaks until indentation is handled.
    fn can_break(&self, node: SyntaxNode<'t>) -> bool {
        let range = node.text_range();
        if node.text().contains(['\n', '\r']) || !self.free_at(range.start()) {
            return false;
        }
        let line_start = self.text[..usize::from(range.start())]
            .rfind(['\n', '\r'])
            .map_or(0, |i| i + 1);
        if !TextSize::try_from(line_start).is_ok_and(|start| self.free_at(start)) {
            return false;
        }
        let tokens: Vec<_> = node.tokens().filter(|t| !t.is_trivia()).collect();
        !tokens.iter().enumerate().any(|(i, t)| {
            let leading = (i > 0).then(|| t.leading_trivia()).into_iter().flatten();
            let trailing = (i + 1 < tokens.len()).then(|| t.trailing_trivia());
            leading
                .chain(trailing.into_iter().flatten())
                .any(|t| t.is_comment())
        })
    }

    /// A lambda split at its arrow: what comes before it, the arrow, and the
    /// body. `None` when the gap after the arrow isn't one the spacing rules
    /// put a space in, so the lambda printed flat matches the fallback.
    #[allow(clippy::type_complexity)]
    fn lambda_parts(
        &self,
        lambda: SyntaxNode<'t>,
    ) -> Option<(Vec<SyntaxElement<'t>>, SyntaxToken<'t>, SyntaxElement<'t>)> {
        let children: Vec<_> = lambda.non_trivia_children().collect();
        let arrow_at = children.iter().position(
            |c| matches!(c, NodeOrToken::Token(t) if matches!(t.text(), "=>" | "\u{21D2}")),
        )?;
        let NodeOrToken::Token(arrow) = children[arrow_at] else {
            return None;
        };
        let [body] = &children[arrow_at + 1..] else {
            return None;
        };
        let next = std::iter::successors(arrow.next_token(), SyntaxToken::next_token)
            .find(|t| !t.is_trivia())?;
        if self.rule_between(&arrow, &next) != Some(true) {
            return None;
        }
        Some((children[..arrow_at].to_vec(), arrow, *body))
    }

    /// The default rule: the node's code children in order, each token with
    /// its comments and whitespace.
    fn fallback(&mut self, node: SyntaxNode<'t>) -> Doc<'t> {
        let children: Vec<_> = node.non_trivia_children().collect();
        Doc::concat(children.into_iter().map(|child| self.element(child)))
    }

    fn element(&mut self, element: SyntaxElement<'t>) -> Doc<'t> {
        match element {
            NodeOrToken::Node(node) => self.node(node),
            NodeOrToken::Token(token) => self.token(token),
        }
    }

    fn token(&mut self, token: SyntaxToken<'t>) -> Doc<'t> {
        self.token_without_trailing(token) + self.trailing(token)
    }

    /// The token with its leading trivia.
    fn token_without_trailing(&mut self, token: SyntaxToken<'t>) -> Doc<'t> {
        let mut docs = Vec::new();
        if self.leading_done == Some(token.text_range().start()) {
            self.leading_done = None;
        } else {
            docs.push(self.leading(token));
        }
        docs.push(self.source(token.text_range(), Piece::Code));
        self.at_file_start = false;
        self.line_breaks = 0;
        Doc::concat(docs)
    }

    /// The token's leading trivia: line breaks, indentation and comments
    /// before it.
    fn leading(&mut self, token: SyntaxToken<'t>) -> Doc<'t> {
        let mut docs = Vec::new();
        let leading: Vec<_> = token.leading_trivia().collect();
        self.trivia_within_lines(&mut docs, &leading, Gap::LineStart, Some(&token));
        Doc::concat(docs)
    }

    /// The leading trivia of the node's first token, printed now so that a
    /// group around the node doesn't contain its line break.
    fn leading_of(&mut self, node: SyntaxNode<'t>) -> Doc<'t> {
        let Some(first) = std::iter::successors(node.first_token(), SyntaxToken::next_token)
            .find(|t| !t.is_trivia())
        else {
            return Doc::Nil;
        };
        // A rule around this one already printed it.
        if self.leading_done == Some(first.text_range().start()) {
            return Doc::Nil;
        }
        let doc = self.leading(first);
        self.leading_done = Some(first.text_range().start());
        doc
    }

    /// Whether the spacing rules put a space between two tokens on the same
    /// line, or `None` if the gap is kept as written.
    fn rule_between(&self, left: &SyntaxToken<'t>, right: &SyntaxToken<'t>) -> Option<bool> {
        let gap = TextRange::new(left.text_range().end(), right.text_range().start());
        if self.gap_is_free(gap) {
            spacing::space_between(left, right)
        } else {
            None
        }
    }

    /// The token's trailing trivia: the gap up to the next token on its line,
    /// or the rest of the line.
    fn trailing(&mut self, token: SyntaxToken<'t>) -> Doc<'t> {
        let start = token.text_range().start();
        if let Some(i) = self.trailing_skipped.iter().position(|&s| s == start) {
            self.trailing_skipped.swap_remove(i);
            return Doc::Nil;
        }
        let mut docs = Vec::new();
        let trailing: Vec<_> = token.trailing_trivia().collect();
        let next = std::iter::successors(token.next_token(), SyntaxToken::next_token)
            .find(|t| !t.is_trivia() && !t.is_missing());
        let next_on_this_line = next.filter(|b| b.leading_trivia().len() == 0);
        let only_spaces = trailing.iter().all(|t| t.kind() == TriviaKind::Whitespace);
        match next_on_this_line {
            Some(next) => {
                match self.rule_between(&token, &next) {
                    // Only spaces before the next code token: the rule decides.
                    Some(space) if only_spaces => {
                        if space {
                            docs.push(Doc::text(" "));
                        }
                    }
                    Some(_) => {
                        self.trivia_within_lines(&mut docs, &trailing, Gap::AfterCode, Some(&next));
                    }
                    None => {
                        for trivia in trailing {
                            docs.push(self.trivia(trivia));
                        }
                    }
                }
            }
            // The end of the line: one space before a comment. Spaces at the
            // very end go; the printer trims them anyway.
            None if only_spaces => {}
            None => self.trivia_within_lines(&mut docs, &trailing, Gap::AfterCode, None),
        }
        Doc::concat(docs)
    }

    /// Pushes `trivia`, which comes before `next`, with scalafmt's spacing
    /// around comments: one space on each side of a comment within a line,
    /// except none before `,`, `;` or `:`. Indentation and anything verbatim
    /// stay as written.
    fn trivia_within_lines(
        &mut self,
        docs: &mut Vec<Doc<'t>>,
        trivia: &[SyntaxTrivia<'t>],
        mut before: Gap,
        next: Option<&SyntaxToken<'t>>,
    ) {
        for (i, &t) in trivia.iter().enumerate() {
            let start = t.text_range().start();
            match t.kind() {
                TriviaKind::Newline => {
                    if self.is_free(start) {
                        self.line_breaks += 1;
                        // The code after the line breaks decides how many blank
                        // lines it may have before it. A comment between them
                        // belongs to that code.
                        let blank_lines = match next {
                            Some(next) if !in_template(next) => {
                                self.style.max_blank_lines_in_blocks
                            }
                            _ => self.style.max_blank_lines_in_templates,
                        };
                        // The line break ending a line, then the blank lines.
                        if self.at_file_start || self.line_breaks > blank_lines + 1 {
                            continue;
                        }
                    }
                    docs.push(self.trivia(t));
                    before = Gap::LineStart;
                }
                // A blank line's whitespace, which the printer would trim
                // anyway. Keeping it would make it the next line's
                // indentation when a line break between them is dropped.
                TriviaKind::Whitespace
                    if before == Gap::LineStart
                        && self.is_free(start)
                        && trivia
                            .get(i + 1)
                            .is_some_and(|t| t.kind() == TriviaKind::Newline) => {}
                TriviaKind::Whitespace if before == Gap::LineStart && self.is_free(start) => {
                    match self.reindent.width(t.text_range().end()) {
                        Some(width) => docs.push(Doc::text(" ".repeat(width))),
                        None => docs.push(self.trivia(t)),
                    }
                    before = Gap::Space;
                }
                TriviaKind::Whitespace if before == Gap::LineStart || !self.is_free(start) => {
                    docs.push(self.trivia(t));
                    before = Gap::Space;
                }
                // Within a line: the space, if any, depends on what's around it.
                TriviaKind::Whitespace => {}
                TriviaKind::LineComment | TriviaKind::BlockComment => {
                    let after_code = matches!(before, Gap::AfterCode | Gap::AfterComment);
                    // A comment at column 0 that moves in.
                    if before == Gap::LineStart
                        && let Some(width) = self.reindent.width(start)
                    {
                        docs.push(Doc::text(" ".repeat(width)));
                    }
                    let mut comment = Doc::Nil;
                    if after_code && self.free_at(start) {
                        comment = Doc::text(" ");
                    }
                    comment = comment + self.trivia(t);
                    self.at_file_start = false;
                    self.line_breaks = 0;
                    // A `//` comment after code takes no width, as in scalafmt,
                    // so a long one doesn't make the code on its line break.
                    if after_code && t.kind() == TriviaKind::LineComment && self.is_free(start) {
                        comment = comment.line_suffix();
                    }
                    docs.push(comment);
                    before = Gap::AfterComment;
                }
            }
        }
        // Code at column 0 that moves in.
        if let Some(next) = next
            && before == Gap::LineStart
            && let Some(width) = self.reindent.width(next.text_range().start())
        {
            docs.push(Doc::text(" ".repeat(width)));
        }
        if let Some(next) = next
            && before == Gap::AfterComment
            && !matches!(next.text(), "," | ";" | ":")
            && self.free_at(next.text_range().start())
        {
            docs.push(Doc::text(" "));
        }
    }

    /// Whether a space may be inserted at `offset`: it's not inside a
    /// verbatim range, though it may be at the start of one.
    fn free_at(&self, offset: TextSize) -> bool {
        self.gap_is_free(TextRange::new(offset, offset))
    }

    /// Whether the source at `offset` may be changed: it's not in a verbatim
    /// range.
    fn is_free(&self, offset: TextSize) -> bool {
        self.parse.verbatim_at(offset).is_none()
    }

    /// Whether no verbatim range overlaps `gap`. An empty gap right before a
    /// range, as in `f(` before `"x"`, is outside it.
    fn gap_is_free(&self, gap: TextRange) -> bool {
        self.parse
            .verbatim_at(gap.start())
            .is_none_or(|v| v.range.start() >= gap.end())
    }

    fn trivia(&mut self, trivia: SyntaxTrivia<'t>) -> Doc<'t> {
        let piece = match trivia.kind() {
            TriviaKind::Whitespace => Piece::Whitespace,
            TriviaKind::Newline => Piece::Newline,
            TriviaKind::LineComment | TriviaKind::BlockComment => Piece::Comment,
        };
        self.source(trivia.text_range(), piece)
    }

    /// The document for a stretch of source. Where it overlaps a verbatim
    /// range, the whole range is copied once, when first reached.
    fn source(&mut self, range: TextRange, piece: Piece) -> Doc<'t> {
        let (mut start, end) = (range.start(), range.end());
        let mut docs = Vec::new();
        while start < end {
            if start < self.copied_until {
                start = self.copied_until.min(end);
                continue;
            }
            while self
                .verbatim
                .get(self.next_verbatim)
                .is_some_and(|v| v.range.end() <= start)
            {
                self.next_verbatim += 1;
            }
            match self.verbatim.get(self.next_verbatim) {
                Some(v) if v.range.start() <= start => {
                    docs.push(self.verbatim_range(*v));
                    self.copied_until = v.range.end();
                    self.next_verbatim += 1;
                }
                Some(v) if v.range.start() < end => {
                    docs.push(self.plain(TextRange::new(start, v.range.start()), piece));
                    start = v.range.start();
                }
                _ => {
                    docs.push(self.plain(TextRange::new(start, end), piece));
                    start = end;
                }
            }
        }
        Doc::concat(docs)
    }

    /// A verbatim range, copied. A multi-line comment starting a line that
    /// moves moves all its lines by as much, so a Scaladoc's ` *` lines stay
    /// under its `/**`.
    fn verbatim_range(&self, v: VerbatimRange) -> Doc<'t> {
        let text = &self.text[v.range];
        let start = v.range.start();
        let Some(width) = self
            .reindent
            .width(start)
            .filter(|_| v.reason == VerbatimReason::Comment)
        else {
            return Doc::verbatim(text);
        };
        let line_start = self.text[..usize::from(start)]
            .rfind('\n')
            .map_or(0, |i| i + 1);
        let shift = width as isize - (usize::from(start) - line_start) as isize;
        let mut out = String::with_capacity(text.len());
        for (i, line) in text.split_inclusive('\n').enumerate() {
            if i == 0 || line.trim().is_empty() {
                out.push_str(line);
            } else if shift > 0 {
                out.push_str(&" ".repeat(shift as usize));
                out.push_str(line);
            } else {
                let spaces = line.len() - line.trim_start_matches(' ').len();
                out.push_str(&line[spaces.min(shift.unsigned_abs())..]);
            }
        }
        Doc::verbatim(out)
    }

    fn plain(&self, range: TextRange, piece: Piece) -> Doc<'t> {
        let text = &self.text[range];
        match piece {
            Piece::Newline => Doc::hardline(),
            _ if text.contains(['\n', '\r']) => Doc::verbatim(text),
            _ => Doc::text(text),
        }
    }
}

/// Whether `token` is in a statement of a template: the body of a class,
/// object, trait or enum, an extension's methods, a package, or the file.
/// Otherwise it's in a block, a method body or `case` clauses.
fn in_template(token: &SyntaxToken<'_>) -> bool {
    for node in token.parent_ancestors() {
        match node.kind() {
            SyntaxKind::CompilationUnit
            | SyntaxKind::TemplateBody
            | SyntaxKind::EnumBody
            | SyntaxKind::WithTemplateBody
            | SyntaxKind::ExtensionDefinition => return true,
            SyntaxKind::Block
            | SyntaxKind::IndentedBlock
            | SyntaxKind::IndentedCases
            | SyntaxKind::CaseBlock => return false,
            _ => {}
        }
    }
    true
}

/// The part of a select chain a node continues: the qualifier of a select,
/// the function of a call, or the function given type arguments.
fn chain_spine(node: SyntaxNode<'_>) -> Option<SyntaxElement<'_>> {
    let field = match node.kind() {
        SyntaxKind::FieldExpression => "value",
        SyntaxKind::CallExpression | SyntaxKind::GenericFunction => "function",
        _ => return None,
    };
    node.non_trivia_children()
        .find(|c| c.field_name() == Some(field))
}

/// The operands and operators of an infix chain at one precedence, in
/// order. Nested chains at the same precedence on the associative side are
/// merged: the left one for most operators, the right one for operators
/// ending in `:`, such as `a :: b :: c`.
fn flatten_infix<'t>(
    node: SyntaxNode<'t>,
    operands: &mut Vec<SyntaxElement<'t>>,
    operators: &mut Vec<SyntaxToken<'t>>,
) -> Option<()> {
    let field = |name: &str| {
        node.non_trivia_children()
            .find(|c| c.field_name() == Some(name))
    };
    let (left, NodeOrToken::Token(op), right) =
        (field("left")?, field("operator")?, field("right")?)
    else {
        return None;
    };
    let level = precedence(op.text());
    let right_assoc = op.text().ends_with(':');
    let same = |e: &SyntaxElement<'t>| match e {
        NodeOrToken::Node(n) if n.kind() == SyntaxKind::InfixExpression => n
            .non_trivia_children()
            .find(|c| c.field_name() == Some("operator"))
            .is_some_and(|o| match o {
                NodeOrToken::Token(t) => {
                    precedence(t.text()) == level && t.text().ends_with(':') == right_assoc
                }
                NodeOrToken::Node(_) => false,
            }),
        _ => false,
    };
    match left {
        NodeOrToken::Node(n) if !right_assoc && same(&left) => {
            flatten_infix(n, operands, operators)?
        }
        _ => operands.push(left),
    }
    operators.push(op);
    match right {
        NodeOrToken::Node(n) if right_assoc && same(&right) => {
            flatten_infix(n, operands, operators)?
        }
        _ => operands.push(right),
    }
    Some(())
}

/// How tightly an infix operator binds, lowest first (SLS 6.12.3).
fn precedence(op: &str) -> u8 {
    // An assignment operator, such as `+=`, binds loosest.
    if op.ends_with('=') && !op.starts_with('=') && !matches!(op, "<=" | ">=" | "!=") {
        return 0;
    }
    match op.chars().next() {
        Some('|') => 2,
        Some('^') => 3,
        Some('&') => 4,
        Some('=' | '!') => 5,
        Some('<' | '>') => 6,
        Some(':') => 7,
        Some('+' | '-') => 8,
        Some('*' | '/' | '%') => 9,
        Some(c) if c.is_alphanumeric() || c == '_' || c == '`' || c == '$' => 1,
        _ => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(source: &str) -> String {
        format(source, &FormatOptions::default()).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn trailing_whitespace_is_removed_outside_strings() {
        let source = "val a = 1   \nval s = \"\"\"line   \n  more\"\"\"  \n";
        assert_eq!(
            fmt(source),
            "val a = 1\nval s = \"\"\"line   \n  more\"\"\"\n"
        );
    }

    #[test]
    fn lines_end_in_newline() {
        let crlf = "object A {\r\n  val x = 1\r\n}\r\n";
        assert_eq!(fmt(crlf), "object A {\n  val x = 1\n}\n");
        // Inside a string, a line ending is part of the value and stays.
        let string = "val s = \"\"\"a\r\nb\"\"\"\r\n";
        assert_eq!(fmt(string), "val s = \"\"\"a\r\nb\"\"\"\n");
    }

    #[test]
    fn blank_lines_in_a_row_follow_the_style() {
        let source = concat!(
            "val a = 1\n\n\n\n\nobject B {\n  object C {\n    val c = 1\n\n\n\n    val d = 2\n  }\n\n\n\n",
            "  def f = {\n    val x = 1\n\n\n\n    x\n  }\n}\n",
        );
        let options = FormatOptions::default();
        let with = |templates, blocks| {
            let style = Style {
                max_blank_lines_in_templates: templates,
                max_blank_lines_in_blocks: blocks,
            };
            format_with(source, &options, Rules::Structural, style).unwrap()
        };
        // Templates, nested or not, get the first limit; a method body the
        // second.
        assert_eq!(
            with(2, 1),
            concat!(
                "val a = 1\n\n\nobject B {\n  object C {\n    val c = 1\n\n\n    val d = 2\n  }\n\n\n",
                "  def f = {\n    val x = 1\n\n    x\n  }\n}\n",
            )
        );
        assert_eq!(
            with(1, 0),
            concat!(
                "val a = 1\n\nobject B {\n  object C {\n    val c = 1\n\n    val d = 2\n  }\n\n",
                "  def f = {\n    val x = 1\n    x\n  }\n}\n",
            )
        );
    }

    #[test]
    fn edge_cases() {
        assert_eq!(fmt(""), "");
        assert_eq!(fmt("\n\n"), "");
        assert_eq!(fmt("// only a comment"), "// only a comment\n");
        assert_eq!(fmt("   "), "");
        assert_eq!(fmt("val x = 1\n  "), "val x = 1\n", "a last line of spaces");
        assert_eq!(
            fmt("val x = 1\n\n\n"),
            "val x = 1\n",
            "blank lines at the end"
        );
        assert_eq!(
            fmt("val x = 1"),
            "val x = 1\n",
            "a missing final line break"
        );
    }

    #[test]
    fn syntax_errors_are_refused() {
        let err = format("object A {\n  def f( = 1\n", &FormatOptions::default()).unwrap_err();
        assert!(
            matches!(err, FormatError::SyntaxErrors(ref errors) if !errors.is_empty()),
            "{err}"
        );
        assert!(
            err.to_string()
                .starts_with("not formatting code with syntax errors")
        );
    }

    /// Formats every error-free file in the corpus, checking that each
    /// result is equivalent, never joins lines with code (rules only add
    /// line breaks so far), and is stable when formatted again. Run with:
    /// `cargo test --release corpus -- --ignored --nocapture`
    #[test]
    #[ignore = "slow; runs over modules/ or $SCUFF_CORPUS"]
    fn corpus_formats_every_file() {
        // Files the formatter correctly refuses because tree-sitter-scala
        // parses the formatted code differently, with the reason.
        const KNOWN_REFUSALS: &[(&str, &str)] = &[(
            "scala3/tests/neg/binaryLiterals.scala",
            "the invalid literal `0b` reads as `0` and a name `b`; the space \
             between them makes `b` an operator ending its line, continued by \
             the `0b2` below",
        )];
        let files = crate::lossless_tree::corpus::corpus_files();
        let mut refused = 0;
        let (mut formatted, mut changed, mut skipped, mut failures) = (0, 0, 0, Vec::new());
        let (mut broken, mut lines_added) = (0, 0);
        // Lines with code or comments: rules add them but never join them.
        let lines = |s: &str| s.lines().filter(|l| !l.trim().is_empty()).count();
        for path in &files {
            let Ok(source) = std::fs::read_to_string(path) else {
                continue;
            };
            match format(&source, &FormatOptions::default()) {
                Err(FormatError::SyntaxErrors(_)) => skipped += 1,
                Err(FormatError::NotEquivalent(_))
                    if KNOWN_REFUSALS.iter().any(|(file, _)| path.ends_with(file)) =>
                {
                    refused += 1;
                }
                Err(e) => failures.push(format!("{}: {e}", path.display())),
                Ok(out) => {
                    formatted += 1;
                    changed += usize::from(out != source);
                    if lines(&out) > lines(&source) {
                        broken += 1;
                        lines_added += lines(&out) - lines(&source);
                    }
                    if lines(&out) < lines(&source) {
                        failures.push(format!("{}: joined lines", path.display()));
                    } else if format(&out, &FormatOptions::default()).as_deref() != Ok(out.as_str())
                    {
                        failures.push(format!("{}: not idempotent", path.display()));
                    }
                }
            }
        }
        for failure in failures.iter().take(15) {
            println!("{failure}");
        }
        println!(
            "{} files: {formatted} formatted ({changed} changed, {broken} with {lines_added} lines added), {skipped} skipped for syntax errors, {refused} known refusals, {} failed",
            files.len(),
            failures.len()
        );
        assert!(failures.is_empty());
    }

    /// Formats `source` with structural rules and with only the fallback,
    /// at a width where nothing needs to break, describing the first
    /// difference, if any.
    fn rules_disagree(source: &str) -> Option<String> {
        let options = FormatOptions {
            max_width: 1_000_000,
        };
        let structural = format_with(source, &options, Rules::Structural, STYLE);
        let fallback = format_with(source, &options, Rules::Fallback, STYLE);
        if structural == fallback {
            return None;
        }
        let show = |r: &Result<String, FormatError>| match r {
            Ok(out) => out.clone(),
            Err(e) => format!("error: {e}\n"),
        };
        let (s, f) = (show(&structural), show(&fallback));
        let line = s
            .lines()
            .zip(f.lines())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| s.lines().count().min(f.lines().count()));
        Some(format!(
            "first difference at output line {}:\n  structural: {:?}\n  fallback:   {:?}",
            line + 1,
            s.lines().nth(line).unwrap_or("<end>"),
            f.lines().nth(line).unwrap_or("<end>"),
        ))
    }

    /// Where everything fits on its line, a structural rule must print
    /// exactly what the fallback prints. Checks the inputs of every `.stat`
    /// case.
    #[test]
    fn structural_rules_match_fallback_on_stat_inputs() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fmt");
        let mut failures = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "stat") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for case in text.split("\n<<<").skip(1) {
                let (title, body) = case.split_once('\n').unwrap_or((case, ""));
                let input = body.split("\n>>>").next().unwrap_or("");
                let input = input.split_once("===\n").map_or(input, |(_, rest)| rest);
                if let Some(difference) = rules_disagree(&format!("{input}\n")) {
                    failures.push(format!(
                        "{}: {}\n{difference}",
                        path.display(),
                        title.trim()
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    }

    /// The same check over the corpus. Run with:
    /// `cargo test --release corpus -- --ignored --nocapture`
    #[test]
    #[ignore = "slow; runs over modules/ or $SCUFF_CORPUS"]
    fn corpus_structural_rules_match_fallback() {
        let files = crate::lossless_tree::corpus::corpus_files();
        let mut failures = Vec::new();
        for path in &files {
            let Ok(source) = std::fs::read_to_string(path) else {
                continue;
            };
            if let Some(difference) = rules_disagree(&source) {
                failures.push(format!("{}: {difference}", path.display()));
            }
        }
        for failure in failures.iter().take(15) {
            println!("{failure}");
        }
        println!("{} files, {} differ", files.len(), failures.len());
        assert!(failures.is_empty());
    }

    /// sbt's code is formatted by scalafmt with the configuration scuff
    /// follows, so every change scuff makes to it is a disagreement with
    /// scalafmt. Writes the changed files out to compare, without failing.
    /// Run with:
    /// `cargo test --release corpus_sbt -- --ignored --nocapture`
    #[test]
    #[ignore = "slow; reports differences from scalafmt over modules/sbt"]
    fn corpus_sbt_differences_from_scalafmt() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("modules/sbt");
        // sbt's `project.excludePaths`, and its scripted tests.
        let excluded = [
            "/contraband-scala/",
            "/launcher-package/citest",
            "-test/",
            "/server-test/src/server-test/",
            "/src/sbt-test/",
        ];
        let files: Vec<_> = crate::lossless_tree::corpus::corpus_files()
            .into_iter()
            .filter(|f| f.starts_with(&root))
            .filter(|f| !excluded.iter().any(|e| f.to_string_lossy().contains(e)))
            .collect();
        // Formatted copies of the changed files, to compare with `diff -r`.
        let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/sbt-formatted");
        let _ = std::fs::remove_dir_all(&out_dir);
        let mut changed = 0;
        for path in &files {
            let Ok(source) = std::fs::read_to_string(path) else {
                continue;
            };
            let Ok(out) = format(&source, &FormatOptions::default()) else {
                continue;
            };
            if out != source {
                changed += 1;
                let copy = out_dir.join(path.strip_prefix(&root).unwrap());
                std::fs::create_dir_all(copy.parent().unwrap()).unwrap();
                std::fs::write(copy, out).unwrap();
            }
        }
        println!("formatted copies in {}", out_dir.display());
        println!("{} sbt files, {changed} changed", files.len());
    }
}
