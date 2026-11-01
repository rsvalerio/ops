//! `ops doc-ids`: no tracker or review-rule ids in Rust doc comments.
//!
//! Doc comments are published API documentation, where an id such as a task
//! number means nothing to the reader. Plain `//` comments are not checked.
//! Needs no external tool. The scan is line based: `///` and `//!` lines count
//! (`////` does not), `/** */` blocks and `#[doc]` attributes do not, and a
//! multi-line string with a line that starts `///` reads as a comment.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;

/// Prefixes that mark an id: backlog tasks and the code-review rule families.
/// A closed list, because the shape alone also matches `UTF-8` and `SHA-256`.
const ID_PREFIXES: &[&str] = &[
    "API", "ARCH", "ASYNC", "CL", "CONC", "DUP", "EDITION", "ERR", "FN", "MACRO", "NATS", "OWN",
    "PATTERN", "PERF", "READ", "SEC", "TASK", "TEST", "TIME", "TRAIT", "UNSAFE", "VER",
];

/// One id found in a doc comment.
#[derive(Debug, PartialEq, Eq)]
struct Finding {
    path: String,
    line: usize,
    id: String,
}

fn is_doc_comment(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("//!") || (line.starts_with("///") && !line.starts_with("////"))
}

fn is_id(token: &str) -> bool {
    token.split_once('-').is_some_and(|(prefix, number)| {
        ID_PREFIXES.contains(&prefix)
            && !number.is_empty()
            && number.bytes().all(|b| b.is_ascii_digit())
    })
}

fn ids_in(line: &str) -> impl Iterator<Item = &str> {
    line.split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .filter(|token| is_id(token))
}

fn scan_source(path: &str, source: &str) -> Vec<Finding> {
    source
        .lines()
        .zip(1..)
        .filter(|(line, _)| is_doc_comment(line))
        .flat_map(|(line, number)| {
            ids_in(line).map(move |id| Finding {
                path: path.to_owned(),
                line: number,
                id: id.to_owned(),
            })
        })
        .collect()
}

/// `ops doc-ids`: scan every non-ignored `*.rs` file under `root`, nested
/// workspaces included, and report each id. Returns failure on any id.
///
/// # Errors
///
/// The walk failed, or a file could not be read, either of which would
/// otherwise pass an unscanned file as clean.
pub fn run_doc_ids(root: &Path, w: &mut dyn Write) -> anyhow::Result<ExitCode> {
    let found = ops_text_fixers::discovery::discover(root, false)
        .with_context(|| format!("walking {}", root.display()))?;
    if let Some(first) = found.walk_errors.first() {
        anyhow::bail!(
            "could not walk the whole tree ({} error(s)), first: {first}",
            found.walk_errors.len()
        );
    }
    let mut findings = Vec::new();
    for path in &found.files {
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let shown = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        findings.extend(scan_source(&shown, &String::from_utf8_lossy(&bytes)));
    }
    findings.sort_by(|a, b| (&a.path, a.line, &a.id).cmp(&(&b.path, b.line, &b.id)));
    for f in &findings {
        writeln!(w, "{}:{}: {}", f.path, f.line, f.id)?;
    }
    if findings.is_empty() {
        writeln!(w, "No task or review-rule ids in doc comments.")?;
        return Ok(ExitCode::SUCCESS);
    }
    writeln!(w, "{} id(s) in doc comments.", findings.len())?;
    Ok(ExitCode::FAILURE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(source: &str) -> Vec<(usize, String)> {
        scan_source("f.rs", source)
            .into_iter()
            .map(|f| (f.line, f.id))
            .collect()
    }

    #[test]
    fn finds_ids_in_outer_and_inner_doc_comments() {
        let source = concat!(
            "//! Fix for TASK-12.\n",
            "fn a() {}\n",
            "    /// See SEC-13 / ERR-5: done.\n",
            "fn b() {}\n",
        );
        assert_eq!(
            ids(source),
            [
                (1, "TASK-12".to_owned()),
                (3, "SEC-13".to_owned()),
                (3, "ERR-5".to_owned())
            ]
        );
    }

    #[test]
    fn plain_comments_and_code_are_not_doc_comments() {
        let source = concat!(
            "// TASK-12\n",
            "//// TASK-13\n",
            "let s = \"/// TASK-14\";\n"
        );
        assert!(ids(source).is_empty());
    }

    #[test]
    fn standards_and_unknown_prefixes_are_not_ids() {
        let source = concat!(
            "/// UTF-8, SHA-256, RFC-3339, TASK-, TASK-12a, MY-TASK-12, task-12\n",
            "/// non-TASK-12\n",
        );
        assert!(ids(source).is_empty());
    }

    #[test]
    fn scans_rust_files_recursively_and_skips_ignored_build_output() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("nested/src")).unwrap();
        std::fs::create_dir_all(root.join("target")).unwrap();
        let tagged = "/// TASK-7\n";
        std::fs::write(root.join("nested/src/lib.rs"), tagged).unwrap();
        std::fs::write(root.join("target/gen.rs"), tagged).unwrap();
        std::fs::write(root.join("notes.txt"), tagged).unwrap();

        let mut out = Vec::new();
        assert_eq!(run_doc_ids(root, &mut out).unwrap(), ExitCode::FAILURE);
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("nested/src/lib.rs:1: TASK-7"), "{text}");
        assert!(
            !text.contains("gen.rs") && !text.contains("notes.txt"),
            "{text}"
        );
    }

    #[test]
    fn a_clean_tree_passes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lib.rs"), "/// Plain docs, UTF-8.\n").unwrap();
        let mut out = Vec::new();
        assert_eq!(
            run_doc_ids(dir.path(), &mut out).unwrap(),
            ExitCode::SUCCESS
        );
    }
}
