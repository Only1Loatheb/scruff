use crate::T;
use crate::lossless_tree::{SyntaxKind, SyntaxNode, SyntaxToken};

/// Whether two code tokens on the same line get a space between them, or
/// `None` to keep the gap as written. Most pairs get a space; the exceptions
/// hug: punctuation like `f(a, b)`, `a.b`, `x: Int`, applications like `f(x)`
/// and `List[Int]`, and prefix operators like `-x`.
pub(super) fn space_between(a: &SyntaxToken<'_>, b: &SyntaxToken<'_>) -> Option<bool> {
    // Source tree-sitter didn't split into tokens, such as `end ` in an end
    // marker, can absorb the spaces next to it. Keep those gaps as written.
    // In a string the text is just its content.
    let loose_text = |t: &SyntaxToken<'_>| {
        t.kind() == SyntaxKind::Text
            && !t.parent_ancestors().any(|n| {
                matches!(
                    n.kind(),
                    SyntaxKind::InterpolatedString | SyntaxKind::String
                )
            })
    };
    if loose_text(a) || loose_text(b) {
        return None;
    }
    // A backticked or symbolic name starting a line, followed by a space and
    // an expression, is a Scala 3 leading infix operator: adding the space in
    // `` `op`{x} `` would join the line to the one before. Keep what was
    // written.
    if a.is_first_on_line()
        && (a.text().starts_with('`') || a.kind() == SyntaxKind::OperatorIdentifier)
    {
        return None;
    }
    // After a prefix operator: `- 42` negates `42`, while `-42` is a single
    // literal, and `- 2.abs` differs from `-2.abs`. Keep what was written.
    if a.parent()
        .is_some_and(|p| p.kind() == SyntaxKind::PrefixExpression && p.first_token() == Some(*a))
    {
        return None;
    }
    Some(decide(a, b))
}

fn decide(a: &SyntaxToken<'_>, b: &SyntaxToken<'_>) -> bool {
    if would_fuse(a.text(), b.text()) {
        return true;
    }
    let (ak, bk) = (a.kind(), b.kind());

    // Inside brackets, and before separators.
    if matches!(ak, T!["("] | T!["["] | T!["."]) || matches!(bk, T![")"] | T!["]"]) {
        return false;
    }
    if matches!(bk, T![","] | T![";"] | T!["."] | T![":"]) {
        return false;
    }

    // Braces get a space inside, except when empty: `{ x }`, `{}`.
    if ak == T!["{"] && bk == T!["}"] {
        return false;
    }
    if ak == T!["${"] || ak == T!["{"] || bk == T!["}"] {
        return true;
    }

    // `'x`, `'{ ... }` quotes, and `@` starting an annotation.
    if ak == T!["'"] || is_first_of(a, SyntaxKind::Annotation) {
        return false;
    }

    // Variance in `[+A, -B]`.
    if let Some(parent) = a.parent()
        && matches!(
            parent.kind(),
            SyntaxKind::CovariantTypeParameter | SyntaxKind::ContravariantTypeParameter
        )
        && parent.first_token() == Some(*a)
    {
        return false;
    }

    // Varargs: `xs*`, `xs: _*`, `f(a ++ b*)`, `args: String*`.
    if b.text() == "*"
        && b.parent().is_some_and(|p| {
            matches!(
                p.kind(),
                SyntaxKind::Vararg | SyntaxKind::RepeatPattern | SyntaxKind::RepeatedParameterType
            )
        })
    {
        return false;
    }

    // Type projection: `A#B`.
    if (ak == T!["#"] || bk == T!["#"])
        && a.parent()
            .is_some_and(|p| p.kind() == SyntaxKind::ProjectedType)
    {
        return false;
    }

    // An access qualifier: `private[sbt]`.
    if b.parent()
        .is_some_and(|p| p.kind() == SyntaxKind::AccessQualifier)
    {
        return false;
    }

    if matches!(bk, T!["("] | T!["["]) {
        return !applies_to(a, b);
    }
    true
}

/// The nodes `token` is the last token of, innermost first. Climbs only
/// while it's the last, so it's quick deep in the tree.
fn ended_by<'t>(token: &SyntaxToken<'t>) -> impl Iterator<Item = SyntaxNode<'t>> + use<'t> {
    let first = token
        .next_sibling_or_token()
        .is_none()
        .then(|| token.parent())
        .flatten();
    std::iter::successors(first, |node| {
        node.next_sibling_or_token()
            .is_none()
            .then(|| node.parent())
            .flatten()
    })
}

/// Whether `a (` or `a [` is an application, written without a space:
/// `f(x)`, `List[Int]`, `f(x)(y)`, `C[V](s)`.
fn applies_to(a: &SyntaxToken<'_>, b: &SyntaxToken<'_>) -> bool {
    // `@Inject() (b: C)`: the parameters after an annotation aren't its
    // arguments; without the space Scala would read them as such.
    if ended_by(a).any(|n| {
        n.kind() == SyntaxKind::Annotation && !n.text_range().contains_range(b.text_range())
    }) {
        return false;
    }
    // An infix operator, as in `xs map (f)`, doesn't apply to what follows.
    if a.field_name() == Some("operator") {
        return false;
    }
    // Constructor parameters after an access modifier, as in
    // `class A private[p] (a: Int)`.
    if ended_by(a).any(|n| n.kind() == SyntaxKind::AccessModifier) {
        return false;
    }
    // A keyword used as a name, such as `def extension(a: Int)`: tree-sitter
    // wraps the keyword token in an identifier node.
    if a.parent()
        .is_some_and(|p| p.kind() == SyntaxKind::Identifier)
    {
        return true;
    }
    match a.kind() {
        T![")"] | T!["]"] | T![this] | T![super] | T!["'"] => true,
        // Arguments after a block argument: `f { a }(b)`.
        T!["}"] => b
            .parent()
            .is_some_and(|p| p.kind() == SyntaxKind::Arguments),
        // A placeholder: `_()`.
        _ if a.parent().is_some_and(|p| p.kind() == SyntaxKind::Wildcard) => true,
        // An operator applies when it's a name, as in `def +(x)`.
        SyntaxKind::OperatorIdentifier => true,
        // Names and literals apply; keywords like `if (` and `extension [`
        // don't.
        kind => kind.is_named(),
    }
}

fn is_first_of(token: &SyntaxToken<'_>, kind: SyntaxKind) -> bool {
    token
        .parent()
        .is_some_and(|p| p.kind() == kind && p.first_token() == Some(*token))
}

/// Whether removing the space between two tokens would make them lex as
/// one, such as `+` and `:` becoming `+:`.
fn would_fuse(a: &str, b: &str) -> bool {
    let (Some(last), Some(first)) = (a.chars().last(), b.chars().next()) else {
        return false;
    };
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let op = |c: char| {
        "!#%&*+-/:<=>?@\\^|~".contains(c)
            || (!c.is_ascii() && !c.is_alphanumeric() && !c.is_whitespace())
    };
    // A name ending in `_` takes the operator after it: `op_ :` is not `op_:`.
    let name_with_underscore = last == '_' && a.chars().count() > 1;
    (word(last) && word(first)) || (op(last) && op(first)) || (name_with_underscore && op(first))
}
