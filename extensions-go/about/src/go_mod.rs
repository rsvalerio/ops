//! Shared `go.mod` parser used by identity and modules providers.
//!
//! Produces module path, Go toolchain version, and the list of local
//! `replace` targets (those with `./...` paths). Single-line and block-form
//! `replace ( ... )` directives are both recognized. Trailing `// ...`
//! comments are stripped from each line before further parsing.

use std::path::Path;

#[derive(Debug, Default)]
pub(crate) struct GoMod {
    pub(crate) module: Option<String>,
    pub(crate) go_version: Option<String>,
    pub(crate) local_replaces: Vec<String>,
}

pub(crate) fn parse(dir: &Path) -> Option<GoMod> {
    let path = dir.join("go.mod");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::debug!(path = %path.display(), error = %e, "failed to read go.mod");
            }
            return None;
        }
    };

    let mut out = GoMod::default();
    let mut in_replace_block = false;

    for raw in content.lines() {
        let line = strip_line_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if in_replace_block {
            if line == ")" {
                in_replace_block = false;
                continue;
            }
            if let Some(target) = parse_replace_directive(line) {
                out.local_replaces.push(target);
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("module ") {
            out.module = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("go ") {
            out.go_version = Some(rest.trim().to_string());
        } else if line == "replace (" || line.starts_with("replace (") {
            in_replace_block = true;
        } else if let Some(rest) = line.strip_prefix("replace ") {
            if let Some(target) = parse_replace_directive(rest) {
                out.local_replaces.push(target);
            }
        }
    }

    Some(out)
}

fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(idx) => &line[..idx],
        None => line,
    }
}

fn parse_replace_directive(rest: &str) -> Option<String> {
    let pos = rest.find("=>")?;
    let target = rest[pos + 2..].trim();
    if target.starts_with("./") {
        Some(target.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_module_and_go_version() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\ngo 1.22\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn strips_trailing_comments() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m // toolchain note\n\ngo 1.22 // toolchain hint\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.module.as_deref(), Some("example.com/m"));
        assert_eq!(m.go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn parses_block_form_replace_directives() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\ngo 1.22\n\nreplace (\n\texample.com/m/api => ./api\n\texample.com/m/sdk => ./sdk\n\texample.com/m/x => github.com/fork/x v1.0.0\n)\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./api", "./sdk"]);
    }

    #[test]
    fn parses_single_line_replace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/m\n\nreplace example.com/m/api => ./api\n",
        )
        .unwrap();
        let m = parse(dir.path()).unwrap();
        assert_eq!(m.local_replaces, vec!["./api"]);
    }

    #[test]
    fn missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse(dir.path()).is_none());
    }
}
