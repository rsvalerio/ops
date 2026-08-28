//! Shared lexical helpers for the `go.mod` and `go.work` parsers.
//!
//! Both files follow the same Go-source comment and block-opener syntax.
//! Centralising these helpers here breaks the prior circular dependency
//! between `go_mod` and `go_work` (ARCH-5 / TASK-1120) and gives future
//! Go-syntax helpers a one-way dependency target.

use std::borrow::Cow;

/// Strip a trailing `// ...` line comment.
///
/// PATTERN-1 / TASK-1107: Go's own `cmd/go` lexer treats `//` as a comment
/// delimiter only when it follows whitespace or starts the line. A bare
/// `line.find("//")` truncates module paths or replace targets that contain
/// a literal `//` (e.g. `module example.com/foo//bar`).
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

/// Match the Go-mod-style `<keyword> (` block opener with optional whitespace
/// between the keyword and the opening paren. Both `use (` and `use(` are
/// accepted by cmd/go; the parser must accept either to avoid silently
/// skipping block-form entries.
pub fn is_block_opener(line: &str, keyword: &str) -> bool {
    let Some(rest) = line.strip_prefix(keyword) else {
        return false;
    };
    let rest = rest.trim_start();
    let Some(after_paren) = rest.strip_prefix('(') else {
        return false;
    };
    // TASK-0994: cmd/go accepts a trailing line comment on the block opener
    // itself (`use ( // members`).
    // PATTERN-1 (TASK-1255): cmd/go also accepts an inline `//` comment with
    // no whitespace between `(` and `//` (`use(// members`, `replace(// note`).
    // The `strip_line_comment` policy (TASK-1107) only fires on `//` at SOL or
    // after whitespace, so the embedded `//` survives the trim and the prior
    // shape returned false. Recognise the no-whitespace inline-comment shape
    // explicitly here so the entire block is not silently dropped.
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
/// PATTERN-1 (TASK-1724): the `go.work` parser previously compared the raw
/// trimmed line against `")"`, so a commented terminator fell through to the
/// directive arm — it was pushed as a use directive named `)` and the block
/// stayed open, absorbing every following top-level line.
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
/// PATTERN-1 (TASK-1727): the go.mod / go.work grammar is a token grammar
/// (`golang.org/x/mod/modfile`), not a line-prefix grammar. The previous
/// `strip_prefix("module ")` shape required exactly one ASCII space, so the
/// tab-separated forms cmd/go accepts (`module\texample.com/m`, `go\t1.22`,
/// `use\t./api`) silently parsed as "no directive at all".
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
/// PATTERN-1 (TASK-1727): modfile lexes Go-style quoted strings, and quoting
/// is *required* for any token containing a space. Left quoted, a `module
/// "example.com/m"` renders as a project named `m"`, a `use "./api"` matches
/// no `tokei_files` row, and a quoted local `replace` target is dropped
/// entirely because it no longer starts with `./`.
///
/// Both the interpreted (`"…"`, with backslash escapes) and raw (`` `…` ``)
/// forms are recognised.
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
/// appears *after* a non-dot, non-empty segment. The leading run of `.`/`..`
/// prefix segments is allowed, because cmd/go accepts `../../shared`.
///
/// SEC-14: shared by `go_mod::parse_replace_directive` (TASK-1212) and
/// `modules::unit_from_use_dir` (TASK-1721) so `replace` targets and `use`
/// directives enforce the same traversal policy as `resolve_member_globs`
/// in `extensions/about/src/workspace.rs` (TASK-1071). `Path::join` does not
/// normalise `..`, so without this check a directive like `./api/../../../etc`
/// resolves outside the project root at the OS layer.
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

    /// PATTERN-1 / TASK-1107: unit-level coverage for the strip helper —
    /// `//` only delimits a trailing comment at start-of-line or after
    /// whitespace; it must pass through when embedded mid-token.
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

    /// PATTERN-1 (TASK-1255): cmd/go accepts an inline `//` comment
    /// immediately after `(` with no whitespace separator. The previous
    /// shape rejected this and silently dropped the entire block.
    #[test]
    fn is_block_opener_accepts_inline_comment_after_paren_no_whitespace() {
        assert!(is_block_opener("use(//note", "use"));
        assert!(is_block_opener("replace(//note", "replace"));
        // Spacing variants still work.
        assert!(is_block_opener("use(// note", "use"));
        assert!(is_block_opener("use ( //note", "use"));
        // The `strip_line_comment` policy (TASK-1107) for embedded `//` in
        // tokens is unchanged: a non-block line with `//` mid-token still
        // does not match.
        assert!(!is_block_opener("use ./mod//x", "use"));
    }

    /// PATTERN-1 (TASK-1724): the block terminator may carry a trailing
    /// comment in either spacing; a bare `== ")"` comparison missed both.
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

    /// PATTERN-1 (TASK-1727): arbitrary whitespace separates verb from
    /// argument; a verb is only a verb on a whitespace boundary.
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

    /// PATTERN-1 (TASK-1727): quoted tokens are what cmd/go actually emits
    /// for any path containing a space; they must be unquoted before use.
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

    /// SEC-14 (TASK-1212 / TASK-1721): `..` past a real segment is traversal;
    /// a leading run of `..` is legal cmd/go input.
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
