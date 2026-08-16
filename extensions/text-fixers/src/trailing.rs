//! Strip trailing spaces and tabs from every line. Preserves the original
//! line terminator (LF or CRLF). Returns `None` when the input is already
//! clean so callers can skip the rewrite and preserve mtimes.

#[must_use]
pub fn fix_trailing(input: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len());
    let mut changed = false;
    let mut start = 0usize;

    while start < input.len() {
        let nl = input[start..].iter().position(|&b| b == b'\n');
        let (line_end, next_start) = match nl {
            Some(off) => (start + off, start + off + 1),
            None => (input.len(), input.len()),
        };

        let has_crlf = line_end > start && input[line_end - 1] == b'\r';
        let content_end = if has_crlf { line_end - 1 } else { line_end };

        let mut trim_to = content_end;
        while trim_to > start {
            let b = input[trim_to - 1];
            if b == b' ' || b == b'\t' {
                trim_to -= 1;
            } else {
                break;
            }
        }

        if trim_to != content_end {
            changed = true;
        }

        out.extend_from_slice(&input[start..trim_to]);
        if has_crlf {
            out.extend_from_slice(b"\r\n");
        } else if nl.is_some() {
            out.push(b'\n');
        }

        start = next_start;
    }

    if changed {
        Some(out)
    } else {
        None
    }
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
}
