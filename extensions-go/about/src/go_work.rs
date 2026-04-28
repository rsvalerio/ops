//! Shared `go.work` parser used by identity and modules providers.
//!
//! Returns the list of `use` directives (block or single-line form) verbatim
//! from the file. Comment-only and empty lines are skipped. Returns `None` if
//! the file is missing or no `use` entries are found.

use std::path::Path;

pub(crate) fn parse_use_dirs(root: &Path) -> Option<Vec<String>> {
    let path = root.join("go.work");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::debug!(path = %path.display(), error = %e, "failed to read go.work");
            }
            return None;
        }
    };
    let mut dirs = Vec::new();
    let mut in_use_block = false;

    for raw in content.lines() {
        let line = raw.trim();
        if line == "use (" {
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

fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
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

    #[test]
    fn strips_inline_comment_in_single_line_use() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\nuse ./mymod // note\n").unwrap();
        let dirs = parse_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./mymod"]);
    }
}
