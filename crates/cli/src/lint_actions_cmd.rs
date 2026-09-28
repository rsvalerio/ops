//! `ops lint-actions`: the GitHub workflow supply-chain policy (TASK-2328).
//!
//! Two rules over every `.github/workflows/*.yml` / `*.yaml`:
//!
//! - every `uses:` is pinned to a full 40-hex commit SHA with a trailing
//!   `# vX.Y.Z` comment naming the version. A tag is repointable by its
//!   maintainer, or by anyone who compromises the upstream repo, with no
//!   visible change here; the comment keeps the pin reviewable. Local
//!   references (`./…`) and prefixes allow-listed under `[lint_actions]
//!   allow` in `.ops.toml` or with `--allow` are exempt; `docker://` images must carry a
//!   `@sha256:` digest.
//! - no job forwards every repository secret with `secrets: inherit`.
//!
//! The scan is line-based, like the grep guard it replaces: a `uses:` key
//! written inside a `run: |` block would be read as a reference too.
//! Symlinked workflow files are skipped — the scan never follows a link out
//! of the workspace.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Context;

const WORKFLOWS_DIR: &str = ".github/workflows";

/// One policy violation: 1-based line and what is wrong.
#[derive(Debug, PartialEq, Eq)]
struct Finding {
    line: usize,
    message: String,
}

/// `ops lint-actions`: lint every workflow under `root`, print one
/// `path:line: message` per violation, and fail when there is any.
///
/// # Errors
///
/// The workflows directory or a workflow file cannot be read.
pub fn run_lint_actions(root: &Path, allow: &[String]) -> anyhow::Result<ExitCode> {
    let files = workflow_files(&root.join(WORKFLOWS_DIR))?;
    let mut out = std::io::stdout().lock();
    if files.is_empty() {
        writeln!(out, "no workflows under {WORKFLOWS_DIR}; nothing to lint")?;
        return Ok(ExitCode::SUCCESS);
    }
    let mut count = 0usize;
    for path in &files {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .display()
            .to_string();
        for f in lint(&text, allow) {
            count = count.saturating_add(1);
            writeln!(out, "{}:{}: {}", safe(&rel), f.line, safe(&f.message))?;
        }
    }
    if count == 0 {
        writeln!(
            out,
            "{} workflow file(s): every action is SHA-pinned and no job uses `secrets: inherit`",
            files.len()
        )?;
        Ok(ExitCode::SUCCESS)
    } else {
        writeln!(out, "{count} workflow policy violation(s)")?;
        Ok(ExitCode::FAILURE)
    }
}

/// Regular `*.yml` / `*.yaml` files directly in `dir`, sorted. A missing
/// directory is an empty list.
fn workflow_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", dir.display())),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.with_context(|| format!("reading {}", dir.display()))?;
        // `DirEntry::file_type` does not follow symlinks.
        if !entry.file_type()?.is_file() {
            continue;
        }
        let path = entry.path();
        if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("yml" | "yaml")
        ) {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Every violation in one workflow's text.
fn lint(text: &str, allow: &[String]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let problem = uses_value(line)
            .and_then(|(value, rest)| uses_problem(value, rest, allow))
            .or_else(|| {
                is_secrets_inherit(line).then(|| {
                    "`secrets: inherit` forwards every repository secret; pass only the secrets \
                 the called workflow declares"
                        .to_owned()
                })
            });
        if let Some(message) = problem {
            findings.push(Finding {
                line: index.saturating_add(1),
                message,
            });
        }
    }
    findings
}

/// For a `uses:` line (optionally a list item), the unquoted reference and
/// the text after it.
fn uses_value(line: &str) -> Option<(&str, &str)> {
    let mut rest = line.trim_start();
    if let Some(item) = rest.strip_prefix('-') {
        rest = item.trim_start();
    }
    let rest = rest.strip_prefix("uses:")?.trim_start();
    let quoted = rest
        .strip_prefix('"')
        .map(|body| ('"', body))
        .or_else(|| rest.strip_prefix('\'').map(|body| ('\'', body)));
    Some(match quoted {
        Some((quote, body)) => body.split_once(quote)?,
        None => rest.split_once(char::is_whitespace).unwrap_or((rest, "")),
    })
}

/// Why a `uses:` reference breaks the pin policy, if it does.
fn uses_problem(value: &str, rest: &str, allow: &[String]) -> Option<String> {
    if value.is_empty() {
        return Some("`uses:` has no reference on the same line".to_owned());
    }
    if value.starts_with("./") || allow.iter().any(|p| value.starts_with(p.as_str())) {
        return None;
    }
    if let Some(image) = value.strip_prefix("docker://") {
        let pinned = image
            .rsplit_once("@sha256:")
            .is_some_and(|(_, digest)| is_hex(digest, 64));
        return (!pinned).then(|| format!("`{value}` is not pinned to an image `@sha256:` digest"));
    }
    let Some((_, reference)) = value.rsplit_once('@') else {
        return Some(format!(
            "`{value}` names no ref; pin it to a full commit SHA"
        ));
    };
    if !is_hex(reference, 40) {
        return Some(format!(
            "`{value}` is not pinned to a full 40-hex commit SHA; pin it with a trailing \
             `# vX.Y.Z` comment"
        ));
    }
    let comment = rest.trim_start().strip_prefix('#').map(str::trim);
    if comment.is_none_or(str::is_empty) {
        return Some(format!(
            "`{value}` lacks a trailing `# vX.Y.Z` comment naming the pinned version"
        ));
    }
    None
}

/// `secrets: inherit`, ignoring quotes and a trailing comment.
fn is_secrets_inherit(line: &str) -> bool {
    let Some(value) = line.trim_start().strip_prefix("secrets:") else {
        return false;
    };
    let value = value.split('#').next().unwrap_or_default().trim();
    value.trim_matches(|c| c == '"' || c == '\'') == "inherit"
}

fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn safe(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    ops_core::ui::sanitise_line(text, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHA: &str = "d23441a48e516b6c34aea4fa41551a30e30af803";

    fn lines(text: &str, allow: &[&str]) -> Vec<usize> {
        let allow: Vec<String> = allow.iter().map(|s| (*s).to_owned()).collect();
        lint(text, &allow).into_iter().map(|f| f.line).collect()
    }

    #[test]
    fn sha_pin_with_version_comment_passes() {
        let text = format!(
            "steps:\n  - uses: actions/checkout@{SHA} # v6.1.0\n  - uses: \"a/b/c@{SHA}\" # v1\n"
        );
        assert!(lines(&text, &[]).is_empty());
    }

    #[test]
    fn tag_or_branch_refs_are_rejected() {
        let text = "  - uses: actions/checkout@v4\n    uses: a/b@main # main\n  - uses: a/b\n";
        assert_eq!(lines(text, &[]), [1, 2, 3]);
    }

    #[test]
    fn short_or_uppercase_sha_is_rejected() {
        let text = format!(
            "  - uses: a/b@{} # v1\n  - uses: a/b@{} # v1\n",
            SHA.get(..7).unwrap(),
            SHA.to_uppercase()
        );
        assert_eq!(lines(&text, &[]), [1, 2]);
    }

    #[test]
    fn sha_pin_without_version_comment_is_rejected() {
        let text = format!("  - uses: a/b@{SHA}\n  - uses: a/b@{SHA} #\n");
        let findings = lint(&text, &[]);
        assert_eq!(findings.len(), 2);
        assert!(findings[0].message.contains("# vX.Y.Z"), "{findings:?}");
    }

    #[test]
    fn local_and_allow_listed_refs_are_exempt() {
        let text =
            "    uses: ./.github/workflows/x.yml\n    uses: me/forge/.github/workflows/b.yml@v1\n";
        assert!(lines(text, &["me/forge/"]).is_empty());
        assert_eq!(lines(text, &[]), [2]);
    }

    #[test]
    fn docker_refs_need_a_digest() {
        let digest = "a".repeat(64);
        let text =
            format!("  - uses: docker://alpine:3.20\n  - uses: docker://alpine@sha256:{digest}\n");
        assert_eq!(lines(&text, &[]), [1]);
    }

    #[test]
    fn secrets_inherit_is_rejected_in_any_spelling() {
        let text =
            "    secrets: inherit\n    secrets: \"inherit\" # all\n    secrets:\n      TOKEN: x\n";
        assert_eq!(lines(text, &[]), [1, 2]);
    }

    #[test]
    fn comments_and_other_keys_are_ignored() {
        let text = "# uses: a/b@v1\n  name: uses: a/b@v1\n  # secrets: inherit\n";
        assert!(lines(text, &[]).is_empty());
    }

    #[test]
    fn scans_only_regular_yaml_files_in_the_workflows_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(workflow_files(&dir.path().join(WORKFLOWS_DIR))
            .unwrap()
            .is_empty());
        let wf = dir.path().join(WORKFLOWS_DIR);
        std::fs::create_dir_all(&wf).unwrap();
        std::fs::write(wf.join("b.yaml"), "").unwrap();
        std::fs::write(wf.join("a.yml"), "").unwrap();
        std::fs::write(wf.join("notes.md"), "").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(wf.join("a.yml"), wf.join("link.yml")).unwrap();
        let names: Vec<String> = workflow_files(&wf)
            .unwrap()
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["a.yml", "b.yaml"]);
    }

    #[test]
    fn run_fails_on_violation_and_passes_when_clean() {
        let dir = tempfile::tempdir().unwrap();
        let wf = dir.path().join(WORKFLOWS_DIR);
        std::fs::create_dir_all(&wf).unwrap();
        std::fs::write(wf.join("ci.yml"), format!("  - uses: a/b@{SHA} # v1\n")).unwrap();
        assert_eq!(
            run_lint_actions(dir.path(), &[]).unwrap(),
            ExitCode::SUCCESS
        );
        std::fs::write(wf.join("ci.yml"), "    secrets: inherit\n").unwrap();
        assert_eq!(
            run_lint_actions(dir.path(), &[]).unwrap(),
            ExitCode::FAILURE
        );
    }

    /// The repository's own workflows satisfy the policy under the
    /// allow-list the CI Workflow Guard job passes.
    #[test]
    fn this_repository_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let allow = ["rsvalerio/forge/".to_owned()];
        for path in workflow_files(&root.join(WORKFLOWS_DIR)).unwrap() {
            let text = std::fs::read_to_string(&path).unwrap();
            let findings = lint(&text, &allow);
            assert!(findings.is_empty(), "{}: {findings:?}", path.display());
        }
    }
}
