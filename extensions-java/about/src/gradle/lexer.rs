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

/// Extract every quoted token from a comma-separated list of values:
/// `'a', 'b', "c"`. Pushes each unquoted token into `out`.
///
/// PATTERN-1 (TASK-0630): when a malformed remainder is encountered (a bare
/// token without an opening quote, or an unbalanced opening quote), log at
/// `tracing::debug` so a partially-parsed include is visible. Tokens already
/// pushed are kept (best-effort recovery, matching the surrounding parser).
pub(super) fn extract_quoted_list(s: &str, out: &mut Vec<String>) {
    let original = s;
    let mut rest = strip_trailing_comment(s).trim();
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
pub(super) fn split_at_unquoted_close_paren(s: &str) -> Option<(&str, &str)> {
    let mut quote: Option<u8> = None;
    for (i, b) in s.bytes().enumerate() {
        match quote {
            Some(q) => {
                if b == q {
                    quote = None;
                }
            }
            None => match b {
                b'"' | b'\'' => quote = Some(b),
                // `)` is ASCII, so `i` and `i + 1` are always char
                // boundaries here and `saturating_add` is exactly `+ 1`; `?`
                // can only bail on an impossible state, and does so as "no
                // closing paren found".
                b')' => return Some((s.get(..i)?, s.get(i.saturating_add(1)..)?)),
                _ => {}
            },
        }
    }
    None
}

/// Strip a trailing `// ...` Groovy/Kotlin comment from a line fragment.
pub(super) fn strip_trailing_comment(s: &str) -> &str {
    s.split_once("//").map_or(s, |(before, _)| before)
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
