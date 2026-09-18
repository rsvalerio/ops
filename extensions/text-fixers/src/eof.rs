//! Ensure the file ends with exactly one newline. Empty files are left alone.
//! CRLF-dominant files keep CRLF; otherwise LF is appended.
//!
//! # Terminator style vs. body style
//!
//! The terminator choice is made from the *input as received* — including the
//! trailing newline run — never from the terminator-stripped body. A file
//! whose only newlines are the trailing ones (`abc\r\n`, the single-line
//! CRLF file a Windows-authored config or a `.bat` script is) has a body with
//! no `\n` at all, so asking the body would always answer LF and rewrite the
//! file's line ending — a conversion, not a whitespace fix. `trailing.rs`'s
//! module docs state the crate's position: treating `\r` as convertible
//! payload is out of scope. A file with *no* terminator anywhere is genuinely
//! ambiguous; LF is appended there, as the repository default.
//!
//! # Lone CR terminators
//!
//! A trailing terminator run made only of `\r` bytes is a lone-CR (classic
//! Mac) terminator. It is a terminator to preserve, not payload to convert:
//! a single lone CR is left untouched, repeated ones collapse to one `\r`,
//! and a LF-bodied file ending in a lone CR is left alone rather than
//! rewritten. Converting it to LF (`b"abc\r"` -> `b"abc\n"`) would be the
//! same unrequested line-ending conversion this module refuses for CRLF.

#[must_use]
pub fn fix_eof(input: &[u8]) -> Option<Vec<u8>> {
    if input.is_empty() {
        return None;
    }

    let mut end = input.len();
    while end > 0 {
        // `end - 1` is always in bounds: `end` starts at `input.len()` and only
        // shrinks, and `end > 0` is the loop condition, so `saturating_sub(1)`
        // is exactly `- 1`. Stopping on `None` keeps the current `end` instead
        // of panicking.
        let Some(&b) = input.get(end.saturating_sub(1)) else {
            break;
        };
        if b == b'\n' || b == b'\r' {
            end = end.saturating_sub(1);
        } else {
            break;
        }
    }

    // `end <= input.len()` by construction; `?` (i.e. "leave the file alone")
    // is the safe fallback if that invariant ever broke.
    let body = input.get(..end)?;
    // The terminator is detected from the whole input, body *and* trailing
    // newline run — not from the stripped `body`. Asking
    // the stripped `body` instead would see zero newlines for any
    // single-line file and append LF to a CRLF file that was already
    // correct — an unrequested line-ending conversion on the pre-commit
    // path, and one that can never reach a fixed point under
    // `* text eol=crlf` checkouts.
    //
    // READ-5 / TASK-2253: a run made only of `\r` bytes (no `\n` anywhere
    // in it) is a lone-CR terminator, preserved as `\r` — converting it to
    // LF would be the same unrequested line-ending conversion this module
    // refuses for CRLF. Every byte of the run is `\r` or `\n` by
    // construction of the walk above, so "CR-only" is "non-empty and holds
    // no `\n`"; an empty run (no terminator at all) stays on the LF-default
    // path below, and `get` over slicing keeps `clippy::indexing_slicing`
    // clean (`end <= input.len()` is the loop's invariant, so the `None`
    // arm is unreachable).
    let run_is_cr_only = input
        .get(end..)
        .is_some_and(|run| !run.is_empty() && !run.contains(&b'\n'));
    let uses_crlf = detect_crlf(input);
    let terminator: &[u8] = if run_is_cr_only {
        b"\r"
    } else if uses_crlf {
        b"\r\n"
    } else {
        b"\n"
    };

    // `end <= input.len()` and the terminator is at most 2 bytes, so the sum is
    // bounded by `input.len() + 2` and can never saturate a `usize`.
    let mut out = Vec::with_capacity(end.saturating_add(terminator.len()));
    out.extend_from_slice(body);
    out.extend_from_slice(terminator);

    if out == input {
        None
    } else {
        Some(out)
    }
}

fn detect_crlf(input: &[u8]) -> bool {
    let mut lf = 0usize;
    let mut crlf = 0usize;
    for (i, &b) in input.iter().enumerate() {
        if b == b'\n' {
            // Guarded by `i > 0`, so `saturating_sub(1)` is exactly `- 1`; the two
            // counters are bounded by `input.len()` and cannot saturate a `usize`.
            if i > 0 && input.get(i.saturating_sub(1)) == Some(&b'\r') {
                crlf = crlf.saturating_add(1);
            } else {
                lf = lf.saturating_add(1);
            }
        }
    }
    crlf > lf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(s: &str) -> Option<String> {
        fix_eof(s.as_bytes()).map(|v| String::from_utf8(v).unwrap())
    }

    #[test]
    fn empty_unchanged() {
        assert!(fix("").is_none());
    }

    #[test]
    fn missing_newline_added() {
        assert_eq!(fix("hello").unwrap(), "hello\n");
    }

    #[test]
    fn single_newline_unchanged() {
        assert!(fix("hello\n").is_none());
    }

    #[test]
    fn multiple_newlines_collapsed() {
        assert_eq!(fix("hello\n\n\n").unwrap(), "hello\n");
    }

    #[test]
    fn crlf_preserved_when_dominant() {
        assert_eq!(fix("a\r\nb\r\n\r\n").unwrap(), "a\r\nb\r\n");
    }

    /// The single-line CRLF class: the body of these inputs holds no `\n` at
    /// all, so a terminator choice made from the stripped body would always
    /// pick LF and convert the file's line ending.
    #[test]
    fn single_line_crlf_file_already_correct_is_not_a_change() {
        assert_eq!(fix_eof(b"abc\r\n"), None);
    }

    #[test]
    fn single_line_crlf_file_with_extra_terminators_keeps_crlf() {
        assert_eq!(fix_eof(b"abc\r\n\r\n").unwrap(), b"abc\r\n");
    }

    #[test]
    fn only_crlf_terminators_keeps_crlf() {
        assert_eq!(fix_eof(b"\r\n\r\n").unwrap(), b"\r\n");
    }

    /// A file with no terminator anywhere is the genuinely ambiguous case:
    /// LF is the documented choice (see the module header).
    #[test]
    fn terminatorless_crlf_history_still_gets_lf() {
        assert_eq!(fix_eof(b"abc").unwrap(), b"abc\n");
    }

    /// READ-5 / TASK-2253: a lone trailing CR is a terminator to preserve,
    /// not convertible payload — `fix_eof(b"abc\r")` must not rewrite the
    /// file to `b"abc\n"` (see the module header's lone-CR section).
    #[test]
    fn lone_trailing_cr_is_preserved_unchanged() {
        assert_eq!(fix_eof(b"abc\r"), None);
    }

    #[test]
    fn lone_cr_only_file_is_preserved_unchanged() {
        assert_eq!(fix_eof(b"\r"), None);
    }

    /// Repeated lone CRs collapse to a single `\r`, mirroring how repeated
    /// CRLF/LF terminators collapse to one of the dominant style.
    #[test]
    fn repeated_lone_crs_collapse_to_one_cr() {
        assert_eq!(fix_eof(b"abc\r\r\r").unwrap(), b"abc\r");
    }

    /// A LF-bodied file ending in a lone CR already ends with exactly one
    /// terminator; it is left alone rather than converted to end in LF.
    #[test]
    fn lf_body_with_trailing_lone_cr_left_alone() {
        assert_eq!(fix_eof(b"a\nb\r"), None);
    }

    #[test]
    fn crlf_added_when_missing_and_dominant() {
        assert_eq!(fix("a\r\nb").unwrap(), "a\r\nb\r\n");
    }

    #[test]
    fn lf_preferred_when_mixed_lf_majority() {
        assert_eq!(fix("a\nb\nc\r\n").unwrap(), "a\nb\nc\n");
    }

    #[test]
    fn idempotent() {
        let first = fix("hello").unwrap();
        assert!(fix(&first).is_none());
    }

    #[test]
    fn only_newlines_collapsed_to_one() {
        assert_eq!(fix("\n\n\n").unwrap(), "\n");
    }
}
