//! Rewrites to the syntax of a file's dialect, made before formatting:
//! `import a._` to `import a.*`, and `import a.{b => c}` to
//! `import a.{b as c}`, in imports and exports alike, where the dialect
//! allows the new syntax, and back where it doesn't. A rename without
//! braces, `import a.b as c`, gets them: `import a.{b => c}`.
//!
//! Only tokens change; comments and whitespace stay where they are, and
//! code copied verbatim, such as `format: off` blocks, is left alone.
//! `check_equivalent` treats each old form and its new one as the same.

use crate::config::Dialect;
use crate::lossless_tree::{NodeOrToken, SyntaxKind, SyntaxNode, SyntaxToken, TextRange, Tree};

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

    const NEW: Dialect = Dialect {
        star_wildcard_import: true,
        as_for_import_rename: true,
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
            star_wildcard_import: true,
            as_for_import_rename: false,
        };
        assert_eq!(
            rewritten("import a.{b => c, _}\n", star_only),
            "import a.{b => c, *}\n"
        );
        let neither = Dialect {
            star_wildcard_import: false,
            as_for_import_rename: false,
        };
        assert_eq!(rewrite(&parse("import a.{b => c, _}\n"), neither), None);
    }

    #[test]
    fn back_to_scala_2() {
        let scala2 = Dialect {
            star_wildcard_import: false,
            as_for_import_rename: false,
        };
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
    fn format_off_is_left_alone() {
        let source = "// format: off\nimport a._\n// format: on\nimport b._\n";
        assert_eq!(
            rewritten(source, NEW),
            "// format: off\nimport a._\n// format: on\nimport b.*\n"
        );
    }
}
