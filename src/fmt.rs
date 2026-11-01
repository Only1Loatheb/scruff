use std::fmt;

use crate::doc::{Doc, PrintOptions, print};
use crate::lossless_tree::{
    Difference, NodeOrToken, Parse, SyntaxElement, SyntaxError, SyntaxNode, SyntaxToken,
    SyntaxTrivia, TextRange, TextSize, TriviaKind, VerbatimRange, check_equivalent, parse,
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
    let parse = parse(source);
    if !parse.errors().is_empty() {
        return Err(FormatError::SyntaxErrors(parse.errors().to_vec()));
    }
    let doc = to_doc(&parse);
    let print_options = PrintOptions {
        max_width: options.max_width,
        indent: "  ".to_string(),
        line_ending: "\n".to_string(),
    };
    let formatted = print(&doc, &print_options);
    check_equivalent(source, &formatted).map_err(FormatError::NotEquivalent)?;
    Ok(formatted)
}

/// The document for a parsed file, before printing.
pub fn to_doc<'t>(parse: &'t Parse) -> Doc<'t> {
    let mut formatter = Formatter {
        text: parse.syntax().text(),
        verbatim: parse.verbatim_ranges(),
        next_verbatim: 0,
        copied_until: TextSize::from(0),
    };
    let body = formatter.node(parse.syntax());
    let eof: Vec<_> = parse.eof_trivia().map(|t| formatter.trivia(t)).collect();
    body + Doc::concat(eof)
}

/// Builds a [`Doc`] from the tree, copying verbatim ranges as they're
/// reached.
struct Formatter<'t> {
    text: &'t str,
    verbatim: &'t [VerbatimRange],
    /// The first verbatim range that hasn't ended before the current
    /// position.
    next_verbatim: usize,
    /// End of the last verbatim range copied; source before it is done.
    copied_until: TextSize,
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
    /// The default rule: the node's code children in order, each token with
    /// its comments and whitespace.
    fn node(&mut self, node: SyntaxNode<'t>) -> Doc<'t> {
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
        let mut docs = Vec::new();
        for trivia in token.leading_trivia() {
            docs.push(self.trivia(trivia));
        }
        docs.push(self.source(token.text_range(), Piece::Code));
        for trivia in token.trailing_trivia() {
            docs.push(self.trivia(trivia));
        }
        Doc::concat(docs)
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
                    docs.push(Doc::verbatim(&self.text[v.range]));
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

    fn plain(&self, range: TextRange, piece: Piece) -> Doc<'t> {
        let text = &self.text[range];
        match piece {
            Piece::Newline => Doc::hardline(),
            _ if text.contains(['\n', '\r']) => Doc::verbatim(text),
            _ => Doc::text(text),
        }
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
    fn edge_cases() {
        assert_eq!(fmt(""), "");
        assert_eq!(fmt("\n\n"), "\n\n");
        assert_eq!(fmt("// only a comment"), "// only a comment");
        assert_eq!(fmt("   "), "");
        assert_eq!(fmt("val x = 1\n  "), "val x = 1\n", "a last line of spaces");
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

    /// Formats every error-free file in the corpus. Run with:
    /// `cargo test --release corpus -- --ignored --nocapture`
    #[test]
    #[ignore = "slow; runs over modules/ or $OUF_CORPUS"]
    fn corpus_baseline_reproduces_every_file() {
        let files = crate::lossless_tree::corpus::corpus_files();
        let (mut formatted, mut skipped, mut failures) = (0, 0, Vec::new());
        for path in &files {
            let Ok(source) = std::fs::read_to_string(path) else {
                continue;
            };
            match format(&source, &FormatOptions::default()) {
                Err(FormatError::SyntaxErrors(_)) => skipped += 1,
                Err(e) => failures.push(format!("{}: {e}", path.display())),
                Ok(out) => {
                    formatted += 1;
                    let strip = |s: &str| -> Vec<String> {
                        s.split('\n').map(|l| l.trim_end().to_string()).collect()
                    };
                    if strip(&out) != strip(&source) {
                        failures.push(format!(
                            "{}: changed more than trailing whitespace",
                            path.display()
                        ));
                    } else if format(&out, &FormatOptions::default()).as_deref() != Ok(out.as_str())
                    {
                        failures.push(format!("{}: not idempotent", path.display()));
                    }
                }
            }
        }
        for failure in failures.iter().take(10) {
            println!("{failure}");
        }
        println!(
            "{} files: {formatted} formatted, {skipped} skipped for syntax errors, {} failed",
            files.len(),
            failures.len()
        );
        assert!(failures.is_empty());
    }
}
