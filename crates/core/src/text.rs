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
        // checked_neg() returns None only for i64::MIN; format the magnitude
        // via unsigned to avoid the overflow on negation.
        let magnitude = match n.checked_neg() {
            Some(positive) => positive.to_string(),
            None => (n.unsigned_abs()).to_string(),
        };
        return format!("-{}", insert_thousands_separators(&magnitude));
    }
    insert_thousands_separators(&n.to_string())
}

fn insert_thousands_separators(digits: &str) -> String {
    let mut result = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().rev().enumerate() {
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
///
/// Non-NotFound IO errors (PermissionDenied, IsADirectory, etc.) are logged at
/// `tracing::warn!` so operators can diagnose "manifest exists but is unreadable"
/// without changing log levels. NotFound remains silent — a missing manifest is
/// a normal condition for optional stacks.
pub fn for_each_trimmed_line<F: FnMut(&str)>(path: &Path, mut f: F) -> Option<()> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to read manifest for line iteration"
            );
            return None;
        }
    };
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
    fn format_number_i64_min_does_not_panic() {
        // i64::MIN cannot be negated; ensure we render the magnitude with the
        // standard separator without panicking or wrapping.
        assert_eq!(format_number(i64::MIN), "-9,223,372,036,854,775,808");
    }

    #[test]
    fn format_number_i64_max() {
        assert_eq!(format_number(i64::MAX), "9,223,372,036,854,775,807");
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

    #[cfg(unix)]
    #[test]
    fn for_each_trimmed_line_unreadable_file_returns_none() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("denied.txt");
        std::fs::write(&path, "data").unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&path, perms).unwrap();

        let res = for_each_trimmed_line(&path, |_| {});
        assert!(res.is_none());

        // Restore so tempdir cleanup works.
        let mut restore = std::fs::metadata(&path).unwrap().permissions();
        restore.set_mode(0o644);
        std::fs::set_permissions(&path, restore).unwrap();
    }
}
