//! Decide whether a buffer is text the fixers may rewrite.
//!
//! Two deliberate departures from the "NUL byte in the first 8 KiB" rule used
//! by `git` and `pre-commit-hooks`:
//!
//! 1. **The whole buffer is inspected, not a fixed prefix.** The caller has
//!    already read the file into memory before asking, so a sniff window buys
//!    nothing and costs correctness: a Netpbm image, a PDF, an `.eps`, or a
//!    tar whose first member is text all have an ASCII run longer than 8 KiB
//!    followed by a binary payload. Under a prefix sniff those are classified
//!    as text and then edited byte-wise — `fix_trailing` deletes payload bytes
//!    anywhere in the file, so the corruption is spread through the data and
//!    is not recoverable by inspection. git's own `buffer_is_binary` is
//!    applied to the whole blob it holds; the prefix was never the rule for a
//!    caller in this position.
//! 2. **Valid UTF-8 is required.** Both fixers reason exclusively about ASCII
//!    whitespace (`0x20`, `0x09`, `0x0A`, `0x0D`), so restricting the input to
//!    UTF-8 is a strict safety win: it rules out NUL-free binary formats,
//!    which the NUL test cannot detect at any offset. The cost is that a
//!    legacy single-byte-encoded text file (latin-1, Shift-JIS) is left
//!    untouched instead of fixed. That is the intended trade: not fixing a
//!    file is recoverable, corrupting one is not.

/// Whether `bytes` is text the fixers may safely rewrite.
///
/// Returns `false` for any buffer containing a NUL byte, and for any buffer
/// that is not valid UTF-8. Empty input is text.
#[must_use]
pub fn is_text(bytes: &[u8]) -> bool {
    !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_text() {
        assert!(is_text(b""));
    }

    #[test]
    fn ascii_is_text() {
        assert!(is_text(b"hello\nworld\n"));
    }

    #[test]
    fn utf8_is_text() {
        assert!(is_text("héllo\nビル\n".as_bytes()));
    }

    #[test]
    fn nul_byte_is_not_text() {
        assert!(!is_text(b"hello\0world"));
    }

    #[test]
    fn nul_far_past_the_old_8_kib_sniff_window_is_still_not_text() {
        // The predecessor of this function inspected only the first 8 KiB and
        // called this buffer text; `run_fixer` then rewrote it. Pinning the
        // opposite is the point of the whole-buffer scan.
        let mut v = vec![b'a'; 8 * 1024 + 1];
        *v.last_mut().unwrap() = 0;
        assert!(!is_text(&v));
    }

    #[test]
    fn nul_free_non_utf8_is_not_text() {
        // Latin-1 "café": no NUL anywhere, so the NUL heuristic alone would
        // hand this to the fixers.
        assert!(!is_text(&[b'c', b'a', b'f', 0xE9, b'\n']));
    }
}
