//! Shared `go.work` parser used by identity and modules providers.
//!
//! Returns the list of `use` directives (block or single-line form) verbatim
//! from the file. Comment-only and empty lines are skipped. Returns `None` if
//! the file is missing or no `use` entries are found.

use std::path::Path;

use crate::go_syntax::{is_block_opener, strip_line_comment};

pub(crate) fn parse_use_dirs(root: &Path) -> Option<Vec<String>> {
    let path = root.join("go.work");
    let content = ops_about::manifest_io::read_optional_text(&path, "go.work")?;
    let mut dirs = Vec::new();
    let mut in_use_block = false;

    for raw in content.lines() {
        let line = raw.trim();
        if is_block_opener(line, "use") {
            in_use_block = true;
            continue;
        }
        if in_use_block {
            if line == ")" {
                in_use_block = false;
                continue;
            }
            if line.is_empty() || line.starts_with("//") {
                continue;
            }
            let stripped = strip_line_comment(line).trim();
            if !stripped.is_empty() {
                dirs.push(stripped.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("use ") {
            let dir = strip_line_comment(rest.trim()).trim();
            if !dir.is_empty() && !dir.starts_with('(') {
                dirs.push(dir.to_string());
            }
        }
    }

    if dirs.is_empty() {
        None
    } else {
        Some(dirs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_inline_comment_in_use_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api // legacy\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    /// TASK-0994: cmd/go accepts a trailing line comment on a `use (`
    /// block opener; the parser must too — otherwise the entire workspace
    /// reports as a single-mod project.
    #[test]
    fn block_opener_accepts_trailing_comment_on_use() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse ( // ws-members\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    #[test]
    fn block_opener_accepts_no_space_before_paren() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse(\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    #[test]
    fn strips_inline_comment_in_single_line_use() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\nuse ./mymod // note\n").unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./mymod"]);
    }

    #[test]
    fn parses_multi_use_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n\t./sdk\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd", "./sdk"]);
    }

    #[test]
    fn parses_single_use_line() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\nuse ./mymod\n").unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./mymod"]);
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse_use_dirs(dir.path()).is_none());
    }

    #[test]
    fn empty_use_block_yields_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n)\n").unwrap();
        assert!(parse_use_dirs(dir.path()).is_none());
    }

    #[test]
    fn comments_only_in_use_block_yields_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t// a comment\n\t./real\n\t// another\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./real"]);
    }

    #[test]
    fn empty_file_yields_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "").unwrap();
        assert!(parse_use_dirs(dir.path()).is_none());
    }

    #[test]
    fn blank_lines_in_use_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\n\t./a\n\n\t./b\n\n)\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./a", "./b"]);
    }

    #[test]
    fn multiple_single_line_uses() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\nuse ./first\nuse ./second\n",
        )
        .unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./first", "./second"]);
    }
}
