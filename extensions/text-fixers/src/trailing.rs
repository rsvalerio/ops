//! Strip trailing spaces and tabs from every line. Preserves the original
//! line terminator (LF or CRLF). Returns `None` when the input is already
//! clean so callers can skip the rewrite and preserve mtimes.

#[must_use]
pub fn fix_trailing(input: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len());
    let mut changed = false;
    let mut start = 0usize;

    while start < input.len() {
        // Offset of the next `\n` relative to `start`, same as indexing `input[start..]`.
        let nl = input.iter().skip(start).position(|&b| b == b'\n');
        // `off` is an index into `input[start..]`, so `start + off < input.len()`
        // and `start + off + 1 <= input.len()`: neither sum can saturate a `usize`.
        let (line_end, next_start) = nl.map_or((input.len(), input.len()), |off| {
            let end = start.saturating_add(off);
            (end, end.saturating_add(1))
        });

        // `line_end > start >= 0` guards both reads, so `saturating_sub(1)` here is
        // exactly `- 1`; `content_end` only subtracts when `has_crlf` proved it.
        let has_crlf =
            line_end > start && input.get(line_end.saturating_sub(1)) == Some(&b'\r');
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

        if trim_to != content_end {
            changed = true;
        }

        // `start <= trim_to <= input.len()` by construction; bailing with `None`
        // (i.e. "no change", leaving the file untouched) is the safe fallback if
        // that invariant ever broke, since a short write would corrupt the file.
        out.extend_from_slice(input.get(start..trim_to)?);
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
