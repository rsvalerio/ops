//! Ad-hoc string-lexer helpers for the Gradle DSL parser.
//!
//! Split out from the parent `gradle` module per FN-1 / TASK-0847 so
//! Gradle DSL semantics (settings/properties/build parsers) live in
//! `super::parse` while the quote-aware tokenisation primitives that
//! produce string slices live here. The pom.rs / pom/ split established
//! the same shape for Maven.

/// Extract a quoted string value: `"foo"` or `'foo'`.
///
/// PATTERN-1 (TASK-1047): the closing-quote scan is backslash-aware so that
/// Groovy / Kotlin string literals containing escaped quotes (e.g.
/// `"see \"v2\" docs"`, `'O\'Brien'`) round-trip without silent truncation.
/// `\\` is treated as a literal backslash run (so a trailing `\\` does not
/// escape the closing quote). Inner escape sequences are preserved verbatim
/// in the returned slice — callers downstream of this lexer treat the value
/// as opaque text, so unescaping is intentionally left out.
pub(super) fn extract_quoted(s: &str) -> Option<&str> {
    let s = s.trim();
    let (open, rest) = if let Some(r) = s.strip_prefix('"') {
        ('"', r)
    } else {
        let r = s.strip_prefix('\'')?;
        ('\'', r)
    };
    let end = find_unescaped(rest, open)?;
    rest.get(..end)
}

/// Find the first byte offset of `quote` in `s` that is not preceded by an
/// odd number of backslashes. Returns `None` if no unescaped `quote` exists.
fn find_unescaped(s: &str, quote: char) -> Option<usize> {
    let mut prev_was_backslash = false;
    for (i, c) in s.char_indices() {
        if c == quote && !prev_was_backslash {
            return Some(i);
        }
        // Toggle so that `\\` resets to "not escaping" — a literal backslash
        // followed by a quote is then correctly recognised as a terminator.
        prev_was_backslash = c == '\\' && !prev_was_backslash;
    }
    None
}

/// Walk `s` outside of string literals, calling `f` with the byte offset and
/// character of every character that is **not** inside a quoted span. `f`
/// returns `Some(v)` to stop the scan early and yield `v`.
///
/// READ-6 / TASK-1744: quoted spans are skipped via [`find_unescaped`], so
/// backslash escaping lives in exactly one place. Every structural scanner in
/// this module (paren matching, comment stripping, brace depth) is built on
/// this primitive instead of hand-rolling its own quote rules — the previous
/// divergence made the Kotlin and Groovy spellings of `include` disagree.
///
/// An unterminated string literal ends the scan: everything after the opening
/// quote is inside a string, so there is nothing structural left to find.
fn scan_unquoted<T>(s: &str, mut f: impl FnMut(usize, char) -> Option<T>) -> Option<T> {
    let mut idx = 0_usize;
    loop {
        let rest = s.get(idx..)?;
        let c = rest.chars().next()?;
        if c == '"' || c == '\'' {
            // The quote is ASCII, so `after` is a char boundary and
            // `saturating_add` is exactly `+ 1`.
            let after = idx.saturating_add(c.len_utf8());
            let end = find_unescaped(s.get(after..)?, c)?;
            idx = after.saturating_add(end).saturating_add(c.len_utf8());
            continue;
        }
        if let Some(found) = f(idx, c) {
            return Some(found);
        }
        idx = idx.saturating_add(c.len_utf8());
    }
}

/// Net brace balance of `line`, counting only `{` / `}` that sit outside
/// string literals and outside a trailing `// …` comment.
///
/// CL-3 / TASK-1733: `parse_gradle_build` uses this to tell a top-level
/// `description` assignment from one nested inside a `task` / `subprojects`
/// block.
pub(super) fn brace_delta(line: &str) -> i32 {
    let mut delta = 0_i32;
    let stopped = scan_unquoted(strip_trailing_comment(line), |_, c| {
        match c {
            '{' => delta = delta.saturating_add(1),
            '}' => delta = delta.saturating_sub(1),
            _ => {}
        }
        None::<()>
    });
    debug_assert!(stopped.is_none(), "brace_delta closure never stops early");
    delta
}

/// Extract every quoted token from a comma-separated list of values:
/// `'a', 'b', "c"`. Pushes each unquoted token into `out`.
///
/// PATTERN-1 (TASK-0630): when a malformed remainder is encountered (a bare
/// token without an opening quote, or an unbalanced opening quote), log at
/// `tracing::debug` so a partially-parsed include is visible. Tokens already
/// pushed are kept (best-effort recovery, matching the surrounding parser).
///
/// READ-6 / TASK-1744: this function does **not** strip trailing comments.
/// The include path strips them exactly once, in `parse_include_line`, using
/// the quote-aware [`strip_trailing_comment`]; stripping again here re-ran a
/// naive cut over already-tokenised input and truncated any argument
/// containing `//` (e.g. `include('a//b')`).
pub(super) fn extract_quoted_list(s: &str, out: &mut Vec<String>) {
    let original = s;
    let mut rest = s.trim();
    while !rest.is_empty() {
        let (quote, after) = if let Some(after) = rest.strip_prefix('"') {
            ('"', after)
        } else if let Some(after) = rest.strip_prefix('\'') {
            ('\'', after)
        } else {
            tracing::debug!(
                line = original,
                remainder = rest,
                "extract_quoted_list: bailed on bare (unquoted) token"
            );
            return;
        };
        let Some(end) = find_unescaped(after, quote) else {
            tracing::debug!(
                line = original,
                remainder = rest,
                "extract_quoted_list: bailed on unbalanced quote"
            );
            return;
        };
        // `end` is a char boundary produced by `find_unescaped` and the quote
        // it points at is ASCII, so both slices always exist and `end + 1` is
        // at most `after.len()` — `saturating_add` is exactly `+ 1`. The
        // bail-out is unreachable in practice; it degrades like a malformed
        // remainder (keep what was already pushed) instead of panicking.
        let (Some(token), Some(tail)) = (after.get(..end), after.get(end.saturating_add(1)..))
        else {
            tracing::debug!(
                line = original,
                remainder = rest,
                "extract_quoted_list: bailed on unsliceable remainder"
            );
            return;
        };
        out.push(token.to_string());
        rest = tail.trim_start();
        if let Some(next) = rest.strip_prefix(',') {
            rest = next.trim_start();
        } else {
            break;
        }
    }
}

/// Split a Kotlin DSL `include(...)` argument tail at the matching `)`,
/// ignoring `)` characters that appear inside double or single quotes. Returns
/// `(args_inside, remainder_after_close)` or `None` if no closing paren is
/// found outside of a string.
/// READ-6 / TASK-1744: the quote scan is backslash-aware, sharing
/// [`find_unescaped`] via [`scan_unquoted`]. The previous hand-rolled byte
/// scan treated `\"` as a closing quote, so `include("legacy\")module")`
/// split at the wrong `)` and the module was silently dropped — while the
/// equivalent Groovy bare-include form kept it.
pub(super) fn split_at_unquoted_close_paren(s: &str) -> Option<(&str, &str)> {
    let close = scan_unquoted(s, |i, c| (c == ')').then_some(i))?;
    // `)` is ASCII, so `close` and `close + 1` are char boundaries and
    // `saturating_add` is exactly `+ 1`; `?` can only bail on an impossible
    // state, and does so as "no closing paren found".
    Some((s.get(..close)?, s.get(close.saturating_add(1)..)?))
}

/// Strip a trailing `// ...` Groovy/Kotlin comment from a line fragment.
///
/// READ-6 / TASK-1744: the `//` must sit outside a string literal — a naive
/// `split_once("//")` chops quoted values that contain `//` (a URL, or a
/// module path such as `include('a//b')`). Shares the escape-aware quote scan
/// with [`find_unescaped`] via [`scan_unquoted`], and is now the single point
/// at which the include path strips comments.
pub(super) fn strip_trailing_comment(s: &str) -> &str {
    let cut = scan_unquoted(s, |i, c| {
        let opens_comment = c == '/'
            && s.get(i.saturating_add(1)..)
                .is_some_and(|rest| rest.starts_with('/'));
        opens_comment.then_some(i)
    });
    // `/` is ASCII, so `cut` is a char boundary and the slice always exists;
    // falling back to the whole string just means "no comment stripped".
    cut.map_or(s, |i| s.get(..i).unwrap_or(s))
}

/// Strip a trailing `# ...` or `! ...` java.util.Properties comment.
///
/// READ-2 / TASK-0812: only treat `#` / `!` as a comment introducer when it
/// appears at the start of (the already-trimmed) value or is preceded by
/// whitespace. The Java .properties spec recognises these markers only at the
/// beginning of a logical line, so a real value like `1.0!beta` or
/// `pwd=foo#bar` must round-trip unchanged. The whitespace-prefix relaxation
/// preserves the long-standing `version=1.2 # release` extraction.
pub(super) fn strip_properties_comment(s: &str) -> &str {
    let mut prev_ws = true;
    for (i, b) in s.bytes().enumerate() {
        if (b == b'#' || b == b'!') && prev_ws {
            // `#` / `!` are ASCII, so `i` is a char boundary and the slice
            // always exists; falling back to the whole string just means
            // "no comment stripped", which is the safe degradation.
            return s.get(..i).unwrap_or(s);
        }
        prev_ws = char::from(b).is_whitespace();
    }
    s
}
