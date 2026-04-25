//! Shared text utilities.

use std::path::Path;

/// Capitalize the first character of a string.
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Format a number with comma separators (e.g. 1234 → "1,234").
pub fn format_number(n: i64) -> String {
    if n < 0 {
        return format!("-{}", format_number(-n));
    }
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

/// Extract the last path component as a project name, falling back to `"project"`.
pub fn dir_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
}

/// Read `path` as UTF-8 text and invoke `f` on each line, with surrounding whitespace
/// trimmed. Returns `Some(())` when the file was read, `None` if it was missing or
/// unreadable. Used by line-based manifest parsers (`go.mod`, `go.work`,
/// `gradle.properties`, etc.) to share the read-and-iterate skeleton.
pub fn for_each_trimmed_line<F: FnMut(&str)>(path: &Path, mut f: F) -> Option<()> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        f(line.trim());
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn capitalize_empty() {
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn capitalize_single_char() {
        assert_eq!(capitalize("a"), "A");
    }

    #[test]
    fn capitalize_already_upper() {
        assert_eq!(capitalize("Hello"), "Hello");
    }

    #[test]
    fn capitalize_lowercase() {
        assert_eq!(capitalize("hello"), "Hello");
    }

    #[test]
    fn format_number_zero() {
        assert_eq!(format_number(0), "0");
    }

    #[test]
    fn format_number_small() {
        assert_eq!(format_number(42), "42");
    }

    #[test]
    fn format_number_thousands() {
        assert_eq!(format_number(1234), "1,234");
    }

    #[test]
    fn format_number_millions() {
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

    #[test]
    fn format_number_negative() {
        assert_eq!(format_number(-1234), "-1,234");
    }

    #[test]
    fn dir_name_normal_path() {
        assert_eq!(
            dir_name(&PathBuf::from("/home/user/myproject")),
            "myproject"
        );
    }

    #[test]
    fn dir_name_root_fallback() {
        assert_eq!(dir_name(&PathBuf::from("/")), "project");
    }

    #[test]
    fn dir_name_empty_fallback() {
        assert_eq!(dir_name(&PathBuf::from("")), "project");
    }

    #[test]
    fn for_each_trimmed_line_invokes_per_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data");
        std::fs::write(&path, "  foo  \n\nbar\n").unwrap();
        let mut seen = Vec::new();
        let res = for_each_trimmed_line(&path, |line| seen.push(line.to_string()));
        assert!(res.is_some());
        assert_eq!(seen, vec!["foo", "", "bar"]);
    }

    #[test]
    fn for_each_trimmed_line_missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let res = for_each_trimmed_line(&dir.path().join("nope"), |_| {});
        assert!(res.is_none());
    }
}
