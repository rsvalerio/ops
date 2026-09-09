//! Strip trailing spaces and tabs from every line.
//!
//! Preserves the original line terminator (LF or CRLF). Returns `None` when
//! the input is already clean, so callers can skip the rewrite and preserve
//! mtimes.
//!
//! # Line terminators
//!
//! `\n` is the only line terminator. `\r\n` is recognised and preserved as a
//! unit, but a **bare `\r` is ordinary content**, not a terminator: a CR-only
//! file (classic-Mac exports, some instrument output) is therefore a single
//! line, and whitespace before its interior `\r` bytes is not stripped. Such
//! files are out of scope; treating `\r` as a terminator would mean rewriting
//! line endings, which is a conversion, not a whitespace fix.
//!
//! What matters is that the docs and the code agree. The invariant this module
//! guarantees, and tests as a property, is:
//!
//! > the output contains exactly as many `0x0A` bytes as the input.
//!
//! It previously did not. `has_crlf` was computed from the byte before
//! `line_end`, and in the no-newline-found branch `line_end` is the length of
//! the *file* — so a file ending in a bare `\r` took the CRLF branch and had a
//! two-byte `\r\n` written where the input had one byte, inventing a newline
//! the input never contained. Whether the invented byte appeared depended on
//! whether some other part of the line happened to need trimming, so
//! `"abc \r"` grew while `"abc\r"` did not.

/// One line's scan result, produced by [`scan_line`].
///
/// PERF-3 / TASK-2168: the per-line analysis lives here so the clean-file
/// check pass and the rebuild pass share one definition of "this line's
/// trailing whitespace" and cannot drift.
struct LineSpan {
    /// Exclusive end of the bytes the line keeps after trimming (an index
    /// into `input`; the line starts at the `start` given to [`scan_line`]).
    trim_to: usize,
    /// The line ended in a `\r\n` (kept as a unit, re-emitted verbatim).
    has_crlf: bool,
    /// The line ended in a bare `\n`.
    has_lf: bool,
    /// Offset of the next line's start; equals `input.len()` after the
    /// final line.
    next_start: usize,
    /// Trailing whitespace was found and stripped from this line.
    trimmed: bool,
}

/// Scan the line starting at `start` and report what a rebuild would keep.
fn scan_line(input: &[u8], start: usize) -> LineSpan {
    // Offset of the next `\n` relative to `start`, same as indexing `input[start..]`.
    let nl = input.iter().skip(start).position(|&b| b == b'\n');
    // `off` is an index into `input[start..]`, so `start + off < input.len()`
    // and `start + off + 1 <= input.len()`: neither sum can saturate a `usize`.
    let (line_end, next_start) = nl.map_or((input.len(), input.len()), |off| {
        let end = start.saturating_add(off);
        (end, end.saturating_add(1))
    });

    // `nl.is_some()` is load-bearing: without it the final line of a file
    // whose last byte is `\r` is read as though that `\r` preceded a
    // newline, and the `\r\n` emitted by the rebuild would invent an `0x0A`
    // the input never had. A `\r` is only half of a terminator when the
    // other half was actually found.
    //
    // `line_end > start >= 0` guards both reads, so `saturating_sub(1)` here is
    // exactly `- 1`; `content_end` only subtracts when `has_crlf` proved it.
    let has_crlf =
        nl.is_some() && line_end > start && input.get(line_end.saturating_sub(1)) == Some(&b'\r');
    let content_end = if has_crlf {
        line_end.saturating_sub(1)
    } else {
        line_end
    };

    let mut trim_to = content_end;
    while trim_to > start {
        // `trim_to - 1` is always in bounds (`trim_to <= input.len()` and only
        // shrinks) and `trim_to > start` is the loop condition, so
        // `saturating_sub(1)` is exactly `- 1`; stopping on `None` leaves the
        // line as-is rather than panicking.
        let Some(&b) = input.get(trim_to.saturating_sub(1)) else {
            break;
        };
        if b == b' ' || b == b'\t' {
            trim_to = trim_to.saturating_sub(1);
        } else {
            break;
        }
    }

    LineSpan {
        trim_to,
        has_crlf,
        has_lf: nl.is_some(),
        next_start,
        trimmed: trim_to != content_end,
    }
}

/// PERF-3 / TASK-2168: scan first, allocate second.
///
/// The check pass walks the lines via [`scan_line`] and returns `None`
/// without allocating or copying anything when the input is already clean —
/// which is every file on a passing run, since the fixers walk the whole
/// repository and the steady state after the first fix is that nothing
/// changes. Only a file that actually needs a trim pays for the output
/// buffer.
#[must_use]
pub fn fix_trailing(input: &[u8]) -> Option<Vec<u8>> {
    // Check pass: no heap allocation, no copy — `LineSpan` is stack data and
    // the scan only reads `input`.
    let mut changed = false;
    let mut start = 0usize;
    while start < input.len() {
        let line = scan_line(input, start);
        changed |= line.trimmed;
        start = line.next_start;
    }
    if !changed {
        return None;
    }

    // Rebuild pass: the input is known to change, so the buffer is earned.
    let mut out = Vec::with_capacity(input.len());
    let mut start = 0usize;
    while start < input.len() {
        let line = scan_line(input, start);
        // `start <= trim_to <= input.len()` by construction; bailing with `None`
        // (i.e. "no change", leaving the file untouched) is the safe fallback if
        // that invariant ever broke, since a short write would corrupt the file.
        out.extend_from_slice(input.get(start..line.trim_to)?);
        if line.has_crlf {
            out.extend_from_slice(b"\r\n");
        } else if line.has_lf {
            out.push(b'\n');
        }
        start = line.next_start;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fix(s: &str) -> Option<String> {
        fix_trailing(s.as_bytes()).map(|v| String::from_utf8(v).unwrap())
    }

    #[test]
    fn empty_unchanged() {
        assert!(fix("").is_none());
    }

    #[test]
    fn clean_unchanged() {
        assert!(fix("hello\nworld\n").is_none());
    }

    #[test]
    fn strips_spaces() {
        assert_eq!(fix("a   \nb\n").unwrap(), "a\nb\n");
    }

    #[test]
    fn strips_tabs() {
        assert_eq!(fix("a\t\t\nb\n").unwrap(), "a\nb\n");
    }

    #[test]
    fn strips_mixed_trailing() {
        assert_eq!(fix("a \t \t\nb\n").unwrap(), "a\nb\n");
    }

    #[test]
    fn preserves_crlf() {
        assert_eq!(fix("a   \r\nb\r\n").unwrap(), "a\r\nb\r\n");
    }

    #[test]
    fn preserves_no_trailing_newline() {
        assert_eq!(fix("a   ").unwrap(), "a");
    }

    #[test]
    fn does_not_touch_leading_whitespace() {
        assert!(fix("    indented\n").is_none());
    }

    #[test]
    fn idempotent() {
        let first = fix("a   \nb\t\n").unwrap();
        assert!(fix(&first).is_none(), "second pass must be a no-op");
    }

    #[test]
    fn blank_line_of_spaces_becomes_empty() {
        assert_eq!(fix("a\n   \nb\n").unwrap(), "a\n\nb\n");
    }

    #[test]
    fn trailing_bare_cr_does_not_grow_the_file() {
        // The regression: this used to become "abc\r\n" — the space stripped
        // *and* an LF invented, one byte longer than the input.
        let out = fix_trailing(b"abc \r");
        assert_eq!(out.as_deref(), None, "a bare CR is content, not a newline");
    }

    #[test]
    fn cr_only_file_is_left_alone() {
        // Documented behaviour: no `\n` means one line, whose trailing bytes
        // are `\r` (not space or tab), so there is nothing to strip and
        // nothing is invented.
        assert!(fix("a \rb \r").is_none());
        assert!(fix("abc\r").is_none());
    }

    #[test]
    fn trailing_whitespace_before_a_final_bare_cr_is_still_reachable_via_lf() {
        // A file that mixes a real LF terminator with a final bare CR: the
        // LF-terminated line is trimmed, the CR-terminated remainder is not,
        // and the LF count is unchanged.
        assert_eq!(fix("a  \nb \r").unwrap(), "a\nb \r");
    }

    #[test]
    fn newline_count_is_invariant() {
        // The property the module doc promises: `fix_trailing` never adds or
        // removes an `0x0A`. Covers CR, CRLF, LF, bare, empty and
        // whitespace-only shapes, each with and without a trailing newline.
        let atoms: [&[u8]; 7] = [b"a", b" ", b"\t", b"\n", b"\r", b"\r\n", b""];
        let mut inputs: Vec<Vec<u8>> = Vec::new();
        for a in atoms {
            for b in atoms {
                for c in atoms {
                    let mut v = Vec::new();
                    v.extend_from_slice(a);
                    v.extend_from_slice(b);
                    v.extend_from_slice(c);
                    inputs.push(v);
                }
            }
        }
        for input in inputs {
            let Some(out) = fix_trailing(&input) else {
                continue;
            };
            assert_eq!(
                bytecount(&out, b'\n'),
                bytecount(&input, b'\n'),
                "newline count changed for {input:?} -> {out:?}"
            );
            assert!(
                out.len() <= input.len(),
                "output grew for {input:?} -> {out:?}"
            );
        }
    }

    // A three-byte test fixture does not justify a `bytecount` dependency.
    #[expect(clippy::naive_bytecount, reason = "tiny test-only inputs")]
    fn bytecount(bytes: &[u8], needle: u8) -> usize {
        bytes.iter().filter(|&&b| b == needle).count()
    }
}
