//! Shared lexical helpers for the `go.mod` and `go.work` parsers.
//!
//! Both files follow the same Go-source comment and block-opener syntax.
//! Centralising these helpers here breaks the prior circular dependency
//! between `go_mod` and `go_work` (ARCH-5 / TASK-1120) and gives future
//! Go-syntax helpers a one-way dependency target.

/// Strip a trailing `// ...` line comment.
///
/// PATTERN-1 / TASK-1107: Go's own `cmd/go` lexer treats `//` as a comment
/// delimiter only when it follows whitespace or starts the line. A bare
/// `line.find("//")` truncates module paths or replace targets that contain
/// a literal `//` (e.g. `module example.com/foo//bar`).
pub(crate) fn strip_line_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'/' && bytes[i + 1] == b'/' {
            // `//` qualifies as a comment delimiter only at start-of-line or
            // when the preceding byte is ASCII whitespace.
            if i == 0 || bytes[i - 1].is_ascii_whitespace() {
                return &line[..i];
            }
        }
        i += 1;
    }
    line
}

/// Match the Go-mod-style `<keyword> (` block opener with optional whitespace
/// between the keyword and the opening paren. Both `use (` and `use(` are
/// accepted by cmd/go; the parser must accept either to avoid silently
/// skipping block-form entries.
pub(crate) fn is_block_opener(line: &str, keyword: &str) -> bool {
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
}
