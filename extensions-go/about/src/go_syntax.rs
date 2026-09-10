//! Shared lexical helpers for the `go.mod` and `go.work` parsers.
//!
//! Both manifests use the same Go-source comment, quoting and block syntax.
//! The helpers live in this leaf module so `go_mod` and `go_work` share one
//! implementation of that grammar without depending on each other.

use std::borrow::Cow;

/// Strip a trailing `// ...` line comment.
///
/// `//` delimits a comment only at start-of-line or when it follows
/// whitespace, matching Go's own `cmd/go` lexer. A `//` embedded in a token
/// — `module example.com/foo//bar`, a replace target `./has//double-slash` —
/// is part of that token and is preserved.
pub fn strip_line_comment(line: &str) -> &str {
    // `match_indices` skips overlapping matches, which is harmless here: a
    // skipped `//` at `i + 1` is always preceded by the `/` at `i`, and a `/`
    // never qualifies as the whitespace predecessor required below.
    for (i, _) in line.match_indices("//") {
        // `i` comes from a match on `line` itself, so it is a char boundary
        // and `get` cannot fail; skip rather than panic if that changes.
        let Some(head) = line.get(..i) else {
            continue;
        };
        // `//` qualifies as a comment delimiter only at start-of-line or
        // when the preceding byte is ASCII whitespace.
        if head.as_bytes().last().is_none_or(u8::is_ascii_whitespace) {
            return head;
        }
    }
    line
}

/// Match the Go-mod-style `<keyword> (` block opener.
///
/// cmd/go accepts arbitrary whitespace — including none — between the keyword
/// and the opening paren, and a trailing line comment on the opener itself in
/// either spacing. All four shapes open a block here: `use (`, `use(`,
/// `use ( // members`, `use(// members`.
pub fn is_block_opener(line: &str, keyword: &str) -> bool {
    let Some(rest) = line.strip_prefix(keyword) else {
        return false;
    };
    let rest = rest.trim_start();
    let Some(after_paren) = rest.strip_prefix('(') else {
        return false;
    };
    // `strip_line_comment` only recognises `//` at start-of-line or after
    // whitespace, so the no-whitespace inline-comment form (`use(//members`)
    // is matched explicitly here before falling back to that helper.
    let trimmed_after = after_paren.trim();
    if trimmed_after.is_empty() {
        return true;
    }
    if trimmed_after.starts_with("//") {
        return true;
    }
    // Fall back to the whitespace-prefixed comment form via the shared
    // strip helper: `( // members` → `(`.
    strip_line_comment(after_paren).trim().is_empty()
}

/// Match the `)` terminator of a `go.mod` / `go.work` block, tolerating a
/// trailing line comment in either spacing (`) // members`, `)//members`).
///
/// A line that starts with `)` but carries anything other than a comment
/// after it (`) ./api`) is not a terminator.
pub fn is_block_terminator(line: &str) -> bool {
    let Some(rest) = line.strip_prefix(')') else {
        return false;
    };
    let trimmed = rest.trim();
    if trimmed.is_empty() || trimmed.starts_with("//") {
        return true;
    }
    // Whitespace-prefixed comment form, via the shared strip policy.
    strip_line_comment(rest).trim().is_empty()
}

/// Split a modfile line into its leading `verb` and the remaining arguments,
/// separated by **arbitrary** whitespace.
///
/// The go.mod / go.work grammar (`golang.org/x/mod/modfile`) is a token
/// grammar, not a line-prefix grammar: every whitespace run separates verb
/// from argument, so `module\texample.com/m`, `go\t1.22`, `use\t./api` and
/// `module   example.com/m` are all legal directives and all match here.
///
/// Returns `None` when the line does not begin with `verb` followed by
/// whitespace, so `gopls x` never matches the `go` verb.
pub fn strip_verb<'a>(line: &'a str, verb: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(verb)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    Some(rest.trim())
}

/// Unquote a Go string literal token, returning it borrowed when there is
/// nothing to unquote.
///
/// modfile lexes Go-style string literals, and quoting is *required* for any
/// token containing a space, so every token this crate reads out of a
/// manifest — module path, `go` version, `use` directive, `replace` target —
/// passes through here before it is inspected or compared.
///
/// Both the interpreted (`"…"`, with backslash escapes) and raw (`` `…` ``)
/// forms are recognised. A bare or unbalanced quote is not a literal and is
/// returned verbatim; an unrecognised escape passes its character through
/// rather than failing the whole parse.
pub fn unquote_token(token: &str) -> Cow<'_, str> {
    if let Some(inner) = token.strip_prefix('`').and_then(|t| t.strip_suffix('`')) {
        return Cow::Borrowed(inner);
    }
    let Some(inner) = token.strip_prefix('"').and_then(|t| t.strip_suffix('"')) else {
        return Cow::Borrowed(token);
    };
    if !inner.contains('\\') {
        return Cow::Borrowed(inner);
    }
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            // Covers `\\` and `\"`, and passes anything else through
            // verbatim rather than failing the whole parse.
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    Cow::Owned(out)
}

/// True when `target` (split on `/` and `\\`) contains a `..` segment that
/// appears *after* a non-dot, non-empty segment.
///
/// This is the traversal policy for every filesystem-valued directive in the
/// crate: a leading run of `.` / `..` prefix segments is allowed, because
/// cmd/go accepts `../../shared`, but a `..` past a real segment is
/// traversal. `Path::join` does not normalise `..` and the OS resolves it
/// lexically on open, so `./api/../../../etc` would otherwise reach outside
/// the project root. Both `go_mod::parse_replace_directive` and
/// `modules::unit_from_use_dir` call this, so `replace` targets and `use`
/// directives enforce one policy — the same one `resolve_member_globs`
/// applies in `extensions/about/src/workspace.rs`.
pub fn has_embedded_parent_dir_segment(target: &str) -> bool {
    let mut seen_normal = false;
    for seg in target.split(['/', '\\']) {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            if seen_normal {
                return true;
            }
            continue;
        }
        seen_normal = true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `//` delimits a trailing comment only at start-of-line or after
    /// whitespace; embedded mid-token it is part of the token.
    #[test]
    fn strip_line_comment_only_fires_on_whitespace_or_sol() {
        assert_eq!(strip_line_comment("// just a comment"), "");
        assert_eq!(
            strip_line_comment("module example.com/m // trailing"),
            "module example.com/m ",
        );
        assert_eq!(
            strip_line_comment("module example.com/foo//bar"),
            "module example.com/foo//bar",
        );
        assert_eq!(
            strip_line_comment("replace ex.com/m => ./has//double-slash"),
            "replace ex.com/m => ./has//double-slash",
        );
        assert_eq!(
            strip_line_comment("module example.com/foo//bar // note"),
            "module example.com/foo//bar ",
        );
        assert_eq!(strip_line_comment("go 1.22"), "go 1.22");
    }

    #[test]
    fn is_block_opener_accepts_both_spacings_and_trailing_comment() {
        assert!(is_block_opener("use (", "use"));
        assert!(is_block_opener("use(", "use"));
        assert!(is_block_opener("use ( // members", "use"));
        assert!(is_block_opener("replace (", "replace"));
        assert!(!is_block_opener("use ./mod", "use"));
        assert!(!is_block_opener("require (", "use"));
    }

    /// An inline `//` comment immediately after `(`, with no whitespace
    /// separator, still opens the block — cmd/go accepts that shape.
    #[test]
    fn is_block_opener_accepts_inline_comment_after_paren_no_whitespace() {
        assert!(is_block_opener("use(//note", "use"));
        assert!(is_block_opener("replace(//note", "replace"));
        // Spacing variants.
        assert!(is_block_opener("use(// note", "use"));
        assert!(is_block_opener("use ( //note", "use"));
        // A `//` embedded in a token is not a comment, so a non-block line
        // carrying one does not match either.
        assert!(!is_block_opener("use ./mod//x", "use"));
    }

    /// The block terminator may carry a trailing comment in either spacing;
    /// a `)` followed by anything else is not a terminator.
    #[test]
    fn is_block_terminator_accepts_trailing_comments() {
        assert!(is_block_terminator(")"));
        assert!(is_block_terminator(") // workspace members"));
        assert!(is_block_terminator(")//members"));
        assert!(is_block_terminator(")\t// members"));
        assert!(!is_block_terminator("./api"));
        assert!(!is_block_terminator(") ./api"));
        assert!(!is_block_terminator("use ("));
    }

    /// Arbitrary whitespace separates verb from argument, and a verb is only
    /// a verb on a whitespace boundary.
    #[test]
    fn strip_verb_splits_on_arbitrary_whitespace() {
        assert_eq!(
            strip_verb("module example.com/m", "module"),
            Some("example.com/m")
        );
        assert_eq!(
            strip_verb("module\texample.com/m", "module"),
            Some("example.com/m")
        );
        assert_eq!(strip_verb("go\t1.22", "go"), Some("1.22"));
        assert_eq!(strip_verb("use\t./api", "use"), Some("./api"));
        assert_eq!(
            strip_verb("module   example.com/ws  ", "module"),
            Some("example.com/ws")
        );
        // No whitespace boundary: not this verb.
        assert_eq!(strip_verb("gopls x", "go"), None);
        assert_eq!(strip_verb("module(", "module"), None);
        assert_eq!(strip_verb("replace ex => ./a", "module"), None);
    }

    /// Interpreted and raw Go string literals are unquoted (escapes
    /// resolved); anything that is not a literal passes through untouched.
    #[test]
    fn unquote_token_handles_go_string_literals() {
        assert_eq!(unquote_token("example.com/m"), "example.com/m");
        assert_eq!(unquote_token("\"example.com/m\""), "example.com/m");
        assert_eq!(unquote_token("\"./has space/sub\""), "./has space/sub");
        assert_eq!(unquote_token("`./raw path`"), "./raw path");
        assert_eq!(unquote_token("\"a\\\"b\""), "a\"b");
        assert_eq!(unquote_token("\"a\\\\b\""), "a\\b");
        // Unbalanced / bare quotes pass through untouched.
        assert_eq!(unquote_token("\""), "\"");
        assert_eq!(unquote_token("\"unterminated"), "\"unterminated");
    }

    /// `..` past a real segment is traversal; a leading run of `..` is legal
    /// cmd/go input, and a segment that merely begins with `..` is not.
    #[test]
    fn has_embedded_parent_dir_segment_only_fires_past_leading_prefix() {
        assert!(has_embedded_parent_dir_segment("./foo/../../etc/passwd"));
        assert!(has_embedded_parent_dir_segment("api/../../../etc"));
        assert!(has_embedded_parent_dir_segment(".\\api\\..\\..\\etc"));
        assert!(!has_embedded_parent_dir_segment("../../shared/lib"));
        assert!(!has_embedded_parent_dir_segment("./api"));
        assert!(!has_embedded_parent_dir_segment("..staging/api"));
    }
}
