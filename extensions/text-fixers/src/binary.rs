//! Heuristic binary detection: a NUL byte in the first 8 KiB means binary.
//! Matches the rule used by `git` and `pre-commit-hooks`.

const SNIFF_LEN: usize = 8 * 1024;

#[must_use]
pub fn is_probably_binary(bytes: &[u8]) -> bool {
    let head = if bytes.len() > SNIFF_LEN {
        &bytes[..SNIFF_LEN]
    } else {
        bytes
    };
    head.contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_not_binary() {
        assert!(!is_probably_binary(b""));
    }

    #[test]
    fn ascii_is_not_binary() {
        assert!(!is_probably_binary(b"hello\nworld\n"));
    }

    #[test]
    fn utf8_is_not_binary() {
        assert!(!is_probably_binary("héllo\nビル\n".as_bytes()));
    }

    #[test]
    fn nul_byte_is_binary() {
        assert!(is_probably_binary(b"hello\0world"));
    }

    #[test]
    fn nul_outside_sniff_window_is_not_binary() {
        let mut v = vec![b'a'; SNIFF_LEN + 1];
        *v.last_mut().unwrap() = 0;
        assert!(!is_probably_binary(&v));
    }
}
