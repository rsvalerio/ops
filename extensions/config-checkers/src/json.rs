//! Parse-only JSON validation.
//!
//! Strict mode uses `serde_json`; lenient mode (`allow_json5 = true`) uses
//! the `json5` crate, which is a strict superset of JSONC and additionally
//! accepts unquoted keys, single-quoted strings, hex/`Infinity`/`NaN`
//! numbers, and other JSON5 extensions.
//!
//! Both modes are bounded the same way. Neither materialises the document —
//! validation deserializes [`serde::de::IgnoredAny`], which drives the same
//! grammar without allocating a `serde_json::Value` tree — and both reject
//! input nested deeper than [`MAX_NESTING_DEPTH`] before the parser sees it.
//! The depth pre-scan is not redundant with the parsers' own behaviour:
//! `json5` 0.4 is a `pest` recursive-descent grammar with no depth bound at
//! all (40 KB of `[[[…]]]` overflows the native stack and aborts the
//! process), and `serde_json`'s `RECURSION_LIMIT` applies to `Value` but not
//! to the iterative skip behind `IgnoredAny`.
//!
//! Both modes also require the input to be UTF-8, which RFC 8259 mandates.
//! That check has to be explicit: `serde_json` validates string contents
//! while building a `Value`, but `IgnoredAny` skips over strings without
//! decoding them, so dropping the tree would otherwise have quietly started
//! accepting files a `Value` parse rejected.

use serde::de::IgnoredAny;

use crate::error::{CheckError, LimitExceeded};

/// Maximum bracket/brace nesting accepted by either JSON mode.
///
/// Matches `serde_json`'s own `RECURSION_LIMIT` so the strict and lenient
/// branches agree on the same input.
pub const MAX_NESTING_DEPTH: u64 = 128;

/// Validate that `bytes` parses as JSON (or JSON5 when `allow_json5`).
///
/// # Errors
/// Returns [`CheckError::InvalidUtf8`] when the input is not UTF-8, or
/// [`CheckError::Parse`] when the parser rejects the input or it nests
/// deeper than [`MAX_NESTING_DEPTH`].
pub fn check_json(bytes: &[u8], allow_json5: bool) -> Result<(), CheckError> {
    let text = std::str::from_utf8(bytes).map_err(CheckError::InvalidUtf8)?;
    if exceeds_depth_limit(bytes, MAX_NESTING_DEPTH) {
        return Err(CheckError::parse(LimitExceeded {
            what: "nesting depth",
            limit: MAX_NESTING_DEPTH,
        }));
    }
    if allow_json5 {
        json5::from_str::<IgnoredAny>(text)
            .map(|_| ())
            .map_err(CheckError::parse)
    } else {
        serde_json::from_str::<IgnoredAny>(text)
            .map(|_| ())
            .map_err(CheckError::parse)
    }
}

/// Lexer state for the depth pre-scan.
enum Scan {
    Structure,
    /// Inside a string literal opened by the held quote byte.
    Str(u8),
    LineComment,
    BlockComment,
}

/// Whether `bytes` nests brackets or braces deeper than `limit`.
///
/// Byte-oriented and iterative on purpose: it must not itself recurse, and
/// it must be able to run before the input is known to be UTF-8. Multi-byte
/// UTF-8 sequences never contain ASCII bytes, so scanning bytes cannot
/// mistake a continuation byte for punctuation.
///
/// String and comment regions are skipped so a `[` inside a quoted value or
/// a JSON5 comment does not count. Single quotes and comments are only
/// meaningful in JSON5, but honouring them in both modes is safe: strict
/// JSON containing either is rejected by `serde_json` regardless, and
/// skipping a region can only ever *lower* the measured depth, never invent
/// one.
fn exceeds_depth_limit(bytes: &[u8], limit: u64) -> bool {
    let mut state = Scan::Structure;
    let mut depth: u64 = 0;
    let mut bytes = bytes.iter().copied().peekable();
    while let Some(b) = bytes.next() {
        match state {
            Scan::Structure => match b {
                b'[' | b'{' => {
                    depth = depth.saturating_add(1);
                    if depth > limit {
                        return true;
                    }
                }
                b']' | b'}' => depth = depth.saturating_sub(1),
                b'"' | b'\'' => state = Scan::Str(b),
                b'/' => match bytes.peek() {
                    Some(b'/') => {
                        bytes.next();
                        state = Scan::LineComment;
                    }
                    Some(b'*') => {
                        bytes.next();
                        state = Scan::BlockComment;
                    }
                    _ => {}
                },
                _ => {}
            },
            Scan::Str(quote) => {
                if b == b'\\' {
                    // Consume the escaped byte so `"\""` does not close early.
                    bytes.next();
                } else if b == quote {
                    state = Scan::Structure;
                }
            }
            Scan::LineComment => {
                if b == b'\n' {
                    state = Scan::Structure;
                }
            }
            Scan::BlockComment => {
                if b == b'*' && bytes.peek() == Some(&b'/') {
                    bytes.next();
                    state = Scan::Structure;
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_json_passes_strict() {
        assert!(check_json(br#"{"a": 1, "b": [true, null]}"#, false).is_ok());
    }

    #[test]
    fn invalid_json_fails_strict() {
        let err = check_json(br#"{"a": }"#, false).unwrap_err();
        assert_eq!(err.to_string(), "expected value at line 1 column 7");
    }

    #[test]
    fn json5_extensions_fail_strict_pass_with_flag() {
        let bytes = br#"{ /* note */ "a": 1, }"#;
        assert!(check_json(bytes, false).is_err());
        assert!(check_json(bytes, true).is_ok());

        // JSON5-only construct (unquoted key) — accepted under allow_json5 to
        // document that the flag is JSON5, not strict JSONC.
        let json5_only = br"{ a: 1 }";
        assert!(check_json(json5_only, false).is_err());
        assert!(check_json(json5_only, true).is_ok());
    }

    #[test]
    fn trailing_characters_after_a_complete_document_are_rejected() {
        // `IgnoredAny` must not turn the strict branch into a prefix parser.
        let err = check_json(br#"{"a": 1} junk"#, false).unwrap_err();
        assert_eq!(err.to_string(), "trailing characters at line 1 column 10");
    }

    #[test]
    fn deeply_nested_input_is_rejected_instead_of_overflowing_the_stack() {
        let depth = 20_000;
        let mut bomb = Vec::with_capacity(depth * 2);
        bomb.extend(std::iter::repeat_n(b'[', depth));
        bomb.extend(std::iter::repeat_n(b']', depth));

        let err = check_json(&bomb, true).unwrap_err();
        assert_eq!(
            err.to_string(),
            "input exceeds the nesting depth limit of 128"
        );
        // The strict branch is bounded by the same pre-scan, not by
        // `serde_json`'s `Value`-only recursion limit.
        let err = check_json(&bomb, false).unwrap_err();
        assert_eq!(
            err.to_string(),
            "input exceeds the nesting depth limit of 128"
        );
    }

    #[test]
    fn nesting_at_the_limit_is_accepted() {
        let depth = usize::try_from(MAX_NESTING_DEPTH).unwrap();
        let mut doc = Vec::new();
        doc.extend(std::iter::repeat_n(b'[', depth));
        doc.extend(std::iter::repeat_n(b']', depth));
        assert!(check_json(&doc, false).is_ok(), "depth {depth} must pass");
    }

    #[test]
    fn brackets_inside_strings_and_comments_do_not_count_towards_depth() {
        let inner = "[[[[[[[[[[".repeat(20); // 200 brackets, never structural
        let quoted = format!("{{\"a\": \"{inner}\"}}");
        assert!(check_json(quoted.as_bytes(), false).is_ok());

        let commented = format!("{{ /* {inner} */ a: 1 }}");
        assert!(check_json(commented.as_bytes(), true).is_ok());
    }

    #[test]
    fn non_utf8_input_reports_invalid_utf8_in_both_modes() {
        // Both branches must reject it, and say the same thing about it. The
        // check is explicit because `IgnoredAny` skips over string bodies
        // without decoding them — `from_str::<IgnoredAny>` alone accepts
        // `["\xff"]`, which a `Value` parse rejected.
        let bytes = b"[\"\xff\"]";
        for allow_json5 in [false, true] {
            let err = check_json(bytes, allow_json5).unwrap_err();
            assert!(
                matches!(err, CheckError::InvalidUtf8(_)),
                "allow_json5={allow_json5}: expected InvalidUtf8, got {err:?}"
            );
            assert!(err.to_string().starts_with("invalid UTF-8: "));
        }
    }

    #[test]
    fn parse_errors_keep_the_underlying_parser_error_as_their_source() {
        use std::error::Error as _;

        let err = check_json(br#"{"a": }"#, false).unwrap_err();
        let source = err.source().expect("Parse must keep its cause");
        assert_eq!(source.to_string(), err.to_string());
        assert!(source.downcast_ref::<serde_json::Error>().is_some());
    }
}
