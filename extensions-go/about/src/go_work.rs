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

    for line in content.lines() {
        let line = line.trim();
        if line == "use (" {
            in_use_block = true;
            continue;
        }
        if in_use_block {
            if line == ")" {
                in_use_block = false;
                continue;
            }
            if !line.is_empty() && !line.starts_with("//") {
                dirs.push(line.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("use ") {
            let dir = rest.trim();
            if !dir.starts_with('(') {
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
