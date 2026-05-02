//! Ad-hoc string-lexer helpers for the Gradle DSL parser.
//!
//! Split out from the parent `gradle` module per FN-1 / TASK-0847 so
//! Gradle DSL semantics (settings/properties/build parsers) live in
//! `super::parse` while the quote-aware tokenisation primitives that
//! produce string slices live here. The pom.rs / pom/ split established
//! the same shape for Maven.

/// Extract a quoted string value: `"foo"` or `'foo'`.
pub(super) fn extract_quoted(s: &str) -> Option<&str> {
    let s = s.trim();
    let (open, rest) = if let Some(r) = s.strip_prefix('"') {
        ('"', r)
    } else if let Some(r) = s.strip_prefix('\'') {
        ('\'', r)
    } else {
        return None;
    };
    let end = rest.find(open)?;
    Some(&rest[..end])
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
        let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            tracing::debug!(
                line = original,
                remainder = rest,
                "extract_quoted_list: bailed on bare (unquoted) token"
            );
            return;
        };
        let after = &rest[1..];
        let Some(end) = after.find(quote) else {
            tracing::debug!(
                line = original,
                remainder = rest,
                "extract_quoted_list: bailed on unbalanced quote"
            );
            return;
        };
        out.push(after[..end].to_string());
        rest = after[end + 1..].trim_start();
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
    let bytes = s.as_bytes();
    let mut quote: Option<u8> = None;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match quote {
            Some(q) => {
                if b == q {
                    quote = None;
                }
            }
            None => match b {
                b'"' | b'\'' => quote = Some(b),
                b')' => return Some((&s[..i], &s[i + 1..])),
                _ => {}
            },
        }
        i += 1;
    }
    None
}

/// Strip a trailing `// ...` Groovy/Kotlin comment from a line fragment.
pub(super) fn strip_trailing_comment(s: &str) -> &str {
    match s.find("//") {
        Some(i) => &s[..i],
        None => s,
    }
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
    let bytes = s.as_bytes();
    let mut prev_ws = true;
    for (i, &b) in bytes.iter().enumerate() {
        if (b == b'#' || b == b'!') && prev_ws {
            return &s[..i];
        }
        prev_ws = (b as char).is_whitespace();
    }
    s
}
