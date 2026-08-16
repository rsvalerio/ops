//! Ensure the file ends with exactly one newline. Empty files are left alone.
//! CRLF-dominant files keep CRLF; otherwise LF is appended.

#[must_use]
pub fn fix_eof(input: &[u8]) -> Option<Vec<u8>> {
    if input.is_empty() {
        return None;
    }

    let mut end = input.len();
    while end > 0 {
        let b = input[end - 1];
        if b == b'\n' || b == b'\r' {
            end -= 1;
        } else {
            break;
        }
    }

    let uses_crlf = detect_crlf(&input[..end.min(input.len())]);
    let terminator: &[u8] = if uses_crlf { b"\r\n" } else { b"\n" };

    let mut out = Vec::with_capacity(end + terminator.len());
    out.extend_from_slice(&input[..end]);
    out.extend_from_slice(terminator);

    if out == input {
        None
    } else {
        Some(out)
    }
}

fn detect_crlf(body: &[u8]) -> bool {
    let mut lf = 0usize;
    let mut crlf = 0usize;
    for (i, &b) in body.iter().enumerate() {
        if b == b'\n' {
            if i > 0 && body[i - 1] == b'\r' {
                crlf += 1;
            } else {
                lf += 1;
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
