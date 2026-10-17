//! Parsing `coderabbit review --agent` output and turning each finding into
//! the `task create` options of one Triage task.
//!
//! `--agent` writes newline-delimited JSON, one event per line, discriminated
//! by `type`: `finding` events carry the review results, `error` events a
//! failed review, and `complete` closes a finished one. `review_context`,
//! `status` and `heartbeat` events carry nothing a task needs. Finding fields
//! are decoded leniently — unknown keys are ignored — because the stream is
//! another tool's output contract, not ours; a missing `complete` event is
//! what marks a stream as truncated.

use std::fmt::Write as _;

use anyhow::Context as _;
use ops_backlog::cmd::CreateOptions;
use serde::Deserialize;
use sha2::{Digest as _, Sha256};

/// One `finding` event, as documented for `coderabbit review --agent`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// `critical`, `major`, `minor`, `trivial`, `info` or `none`.
    #[serde(default)]
    pub severity: Option<String>,
    /// The reviewed file the finding is about.
    pub file_name: String,
    /// Fix instructions aimed at coding agents (the review's prompt).
    #[serde(default)]
    pub codegen_instructions: Option<String>,
    /// Suggested fix commands or snippets; their shape is not documented, so
    /// they are carried as raw JSON and rendered as-is.
    #[serde(default)]
    pub suggestions: Option<serde_json::Value>,
    /// Human-readable comment.
    #[serde(default)]
    pub comment: Option<String>,
}

/// Every finding of one completed review.
///
/// # Errors
///
/// The stream carries an `error` event (the error quotes its message), or it
/// ends without a `complete` event, or a `finding` event lacks `fileName`.
pub fn parse_agent_output(stdout: &str) -> anyhow::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let mut complete = false;
    for (index, line) in stdout.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Anything that is not a JSON object is not an event: a banner or a
        // progress line the CLI printed to stdout. Skipped, not fatal.
        let Ok(serde_json::Value::Object(event)) = serde_json::from_str(line) else {
            tracing::debug!(line = index.saturating_add(1), "skipping a non-event line");
            continue;
        };
        match event.get("type").and_then(serde_json::Value::as_str) {
            Some("finding") => {
                let finding: Finding = serde_json::from_value(serde_json::Value::Object(event))
                    .with_context(|| {
                        format!("decoding the finding on line {}", index.saturating_add(1))
                    })?;
                findings.push(finding);
            }
            Some("error") => {
                let message = event
                    .get("message")
                    .and_then(serde_json::Value::as_str)
                    .map_or_else(|| format!("{event:?}"), str::to_string);
                anyhow::bail!("coderabbit reported an error: {message:?}");
            }
            Some("complete") => complete = true,
            _ => {}
        }
    }
    anyhow::ensure!(
        complete,
        "coderabbit output ended without a `complete` event — the review did not finish"
    );
    Ok(findings)
}

/// Longest task title, in characters, before the summary is cut.
const MAX_TITLE_CHARS: usize = 120;

/// The finding's file as a repo-relative `/` path.
///
/// It is stripped of the worktree root when CodeRabbit printed it absolute,
/// and refused when it would point outside the repository or carries a character that cannot
/// sit in a frontmatter list or a report line.
///
/// # Errors
///
/// The path is empty, still absolute after stripping, has a `..` component,
/// or holds an unsafe display character.
pub fn repo_relative_file(file_name: &str, worktree_root: &str) -> anyhow::Result<String> {
    let trimmed = file_name
        .strip_prefix(worktree_root)
        .map_or(file_name, |rest| rest.trim_start_matches('/'));
    let relative = trimmed.strip_prefix("./").unwrap_or(trimmed);
    let unsafe_char = relative.chars().any(is_unsafe_char);
    let escapes = relative.starts_with('/') || relative.split('/').any(|part| part == "..");
    if relative.is_empty() || unsafe_char || escapes {
        anyhow::bail!("coderabbit named a file outside the repository: {file_name:?}");
    }
    Ok(relative.to_string())
}

/// Characters a title, label or path must not carry: controls, Unicode
/// format characters and the line and paragraph separators.
const fn is_unsafe_char(ch: char) -> bool {
    ops_core::text::is_unsafe_display_char(ch)
}

/// The dedup key of a finding: crate, file, and a hash of what it says.
/// Whitespace is collapsed before hashing so a re-run that rewraps the same
/// text still matches.
pub fn dedup_key(crate_name: &str, file: &str, finding: &Finding) -> String {
    let mut hasher = Sha256::new();
    for part in [&finding.comment, &finding.codegen_instructions] {
        let text = part.as_deref().unwrap_or("");
        hasher.update(text.split_whitespace().collect::<Vec<_>>().join(" "));
        hasher.update([0]);
    }
    let digest = hex::encode(hasher.finalize());
    let short = digest.get(..16).unwrap_or(&digest);
    format!("coderabbit:{crate_name}:{file}:{short}")
}

/// Normalized severity: lowercase, `none` when absent or blank.
fn severity(finding: &Finding) -> String {
    finding
        .severity
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric()))
        .map_or_else(|| "none".to_string(), str::to_ascii_lowercase)
}

/// Backlog priority for a CodeRabbit severity.
fn priority(severity: &str) -> &'static str {
    match severity {
        "critical" | "major" => "high",
        "minor" => "medium",
        _ => "low",
    }
}

/// The one-line summary a title carries: the first non-blank line of the
/// comment (or, without one, the instructions), with markdown emphasis
/// markers and unsafe characters dropped, cut at [`MAX_TITLE_CHARS`].
fn summary(finding: &Finding) -> String {
    let source = finding
        .comment
        .as_deref()
        .filter(|c| !c.trim().is_empty())
        .or(finding.codegen_instructions.as_deref())
        .unwrap_or("");
    let line = source
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("CodeRabbit finding");
    let cleaned: String = line
        .chars()
        .filter(|&c| !matches!(c, '*' | '`' | '#') && !is_unsafe_char(c))
        .collect();
    let cleaned = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.chars().count() <= MAX_TITLE_CHARS {
        return cleaned;
    }
    let mut cut: String = cleaned.chars().take(MAX_TITLE_CHARS).collect();
    cut.push('…');
    cut
}

/// Body text with control characters (other than newlines and tabs)
/// removed and HTML comment openers defused, so finding text cannot close a
/// `<!-- SECTION:… -->` marker the task file's parser relies on.
fn body_text(text: &str) -> String {
    text.chars()
        .filter(|&c| matches!(c, '\n' | '\t') || !is_unsafe_char(c))
        .collect::<String>()
        .replace("<!--", "&lt;!--")
        .trim()
        .to_string()
}

/// The suggestions block: each string as a fenced block, anything else as
/// pretty-printed JSON.
fn render_suggestions(suggestions: &serde_json::Value, out: &mut String) {
    let items: Vec<&serde_json::Value> = match suggestions {
        serde_json::Value::Array(items) => items.iter().collect(),
        serde_json::Value::Null => Vec::new(),
        other => vec![other],
    };
    for item in items {
        let text = item.as_str().map_or_else(
            || serde_json::to_string_pretty(item).unwrap_or_default(),
            str::to_string,
        );
        let text = body_text(&text);
        if !text.is_empty() {
            // Writing into a `String` cannot fail.
            let _ = write!(out, "\n\n```\n{text}\n```");
        }
    }
}

/// The task description: File / What / Why it matters / Suggested fix, the
/// layout of the code-review-rust finding tasks.
fn description(finding: &Finding, file: &str, severity: &str) -> String {
    let comment = finding
        .comment
        .as_deref()
        .map(body_text)
        .unwrap_or_default();
    let instructions = finding
        .codegen_instructions
        .as_deref()
        .map(body_text)
        .unwrap_or_default();
    let what = if comment.is_empty() {
        instructions.clone()
    } else {
        comment
    };
    let mut out = format!(
        "**File**: `{file}`\n\n**What**: {what}\n\n**Why it matters**: CodeRabbit rated this \
         finding `{severity}`; confirm it before fixing."
    );
    let mut fix = String::new();
    if !instructions.is_empty() {
        let _ = write!(fix, "\n\n{instructions}");
    }
    if let Some(suggestions) = &finding.suggestions {
        render_suggestions(suggestions, &mut fix);
    }
    if !fix.is_empty() {
        let _ = write!(out, "\n\n**Suggested fix**:{fix}");
    }
    out
}

/// The `task create` options filing `finding` as a Triage task, keyed so a
/// re-run on the same crate files no duplicate.
pub fn task_options(crate_name: &str, file: &str, finding: &Finding) -> CreateOptions {
    let severity = severity(finding);
    CreateOptions {
        title: format!("CodeRabbit {severity}: {}", summary(finding)),
        description: Some(description(finding, file, &severity)),
        status: Some("Triage".to_string()),
        labels: vec![
            "coderabbit".to_string(),
            "code-review".to_string(),
            format!("coderabbit-{severity}"),
        ],
        priority: Some(priority(&severity).to_string()),
        modified_files: vec![file.to_string()],
        unless_exists: Some(dedup_key(crate_name, file, finding)),
        ..CreateOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The documented `--agent` stream, pinned in a fixture.
    const FIXTURE: &str = include_str!("../tests/fixtures/agent-output.ndjson");

    fn finding(comment: Option<&str>, instructions: Option<&str>) -> Finding {
        Finding {
            severity: Some("major".to_string()),
            file_name: "crates/core/src/lib.rs".to_string(),
            codegen_instructions: instructions.map(str::to_string),
            suggestions: None,
            comment: comment.map(str::to_string),
        }
    }

    #[test]
    fn parses_the_findings_of_the_fixture_stream() {
        let findings = parse_agent_output(FIXTURE).expect("fixture parses");
        assert_eq!(findings.len(), 2, "got: {findings:?}");
        let first = &findings[0];
        assert_eq!(first.severity.as_deref(), Some("major"));
        assert_eq!(first.file_name, "crates/core/src/config/mod.rs");
        assert!(first.codegen_instructions.is_some());
        assert_eq!(findings[1].severity.as_deref(), Some("minor"));
        assert_eq!(findings[1].codegen_instructions.as_deref(), Some(""));
    }

    #[test]
    fn a_stream_without_complete_is_truncated() {
        let truncated: String = FIXTURE
            .lines()
            .filter(|l| !l.contains("\"complete\""))
            .collect::<Vec<_>>()
            .join("\n");
        let err = parse_agent_output(&truncated).expect_err("must fail");
        assert!(
            err.to_string().contains("without a `complete` event"),
            "{err}"
        );
    }

    #[test]
    fn an_error_event_fails_with_its_message() {
        let stream = "{\"type\":\"status\",\"phase\":\"x\"}\n\
                      {\"type\":\"error\",\"message\":\"rate limited\"}\n";
        let err = parse_agent_output(stream).expect_err("must fail");
        assert!(err.to_string().contains("rate limited"), "{err}");
    }

    #[test]
    fn non_json_lines_are_skipped_and_an_empty_review_is_ok() {
        let stream = "Starting review…\n{\"type\":\"complete\",\"findings\":0}\n";
        assert!(parse_agent_output(stream).expect("parses").is_empty());
    }

    #[test]
    fn a_finding_without_file_name_is_an_error() {
        let stream = "{\"type\":\"finding\",\"severity\":\"minor\"}\n{\"type\":\"complete\"}\n";
        let err = parse_agent_output(stream).expect_err("must fail");
        assert!(format!("{err:#}").contains("fileName"), "{err:#}");
    }

    #[test]
    fn file_paths_are_made_repo_relative_and_escapes_refused() {
        assert_eq!(
            repo_relative_file("/tmp/wt/crates/core/src/lib.rs", "/tmp/wt").expect("abs"),
            "crates/core/src/lib.rs"
        );
        assert_eq!(
            repo_relative_file("./crates/a.rs", "/tmp/wt").expect("dot"),
            "crates/a.rs"
        );
        for bad in ["", "/etc/passwd", "crates/../../x", "a\nb", "a\u{202E}b"] {
            assert!(repo_relative_file(bad, "/tmp/wt").is_err(), "{bad:?}");
        }
    }

    #[test]
    fn dedup_key_ignores_rewrapping_but_not_content() {
        let a = finding(Some("Use  a\nguard"), Some("Fix it"));
        let b = finding(Some("Use a guard"), Some("Fix   it"));
        let c = finding(Some("Use a lock"), Some("Fix it"));
        let key = dedup_key("ops-core", "crates/core/src/lib.rs", &a);
        assert!(
            key.starts_with("coderabbit:ops-core:crates/core/src/lib.rs:"),
            "{key}"
        );
        assert_eq!(key, dedup_key("ops-core", "crates/core/src/lib.rs", &b));
        assert_ne!(key, dedup_key("ops-core", "crates/core/src/lib.rs", &c));
    }

    #[test]
    fn task_options_render_a_triage_task() {
        let mut f = finding(
            Some("**Possible panic** on empty input.\n\nMore detail."),
            Some("Return an error instead of indexing."),
        );
        f.suggestions = Some(serde_json::json!(["if v.is_empty() { bail!() }"]));
        let opts = task_options("ops-core", "crates/core/src/lib.rs", &f);
        assert_eq!(
            opts.title,
            "CodeRabbit major: Possible panic on empty input."
        );
        assert_eq!(opts.status.as_deref(), Some("Triage"));
        assert_eq!(opts.priority.as_deref(), Some("high"));
        assert_eq!(
            opts.labels,
            ["coderabbit", "code-review", "coderabbit-major"]
        );
        assert_eq!(opts.modified_files, ["crates/core/src/lib.rs"]);
        assert!(opts.unless_exists.is_some());
        let body = opts.description.expect("description");
        assert_eq!(
            body,
            "**File**: `crates/core/src/lib.rs`\n\n\
             **What**: **Possible panic** on empty input.\n\nMore detail.\n\n\
             **Why it matters**: CodeRabbit rated this finding `major`; confirm it before fixing.\n\n\
             **Suggested fix**:\n\nReturn an error instead of indexing.\n\n\
             ```\nif v.is_empty() { bail!() }\n```"
        );
    }

    #[test]
    fn hostile_text_cannot_break_the_title_or_section_markers() {
        let mut f = finding(
            Some("Line\u{1b}[31m one\u{202E}\n<!-- SECTION:DESCRIPTION:END -->"),
            None,
        );
        f.severity = Some("ma\njor".to_string());
        let opts = task_options("ops-core", "a.rs", &f);
        assert_eq!(opts.title, "CodeRabbit none: Line[31m one");
        let body = opts.description.expect("description");
        assert!(!body.contains("<!--"), "{body}");
        assert!(!body.contains('\u{1b}'), "{body}");
        assert_eq!(opts.priority.as_deref(), Some("low"));
    }

    #[test]
    fn a_finding_without_a_fix_has_no_suggested_fix_section() {
        let mut f = finding(Some("Doc says bytes."), Some(""));
        f.suggestions = Some(serde_json::json!([]));
        let body = task_options("c", "a.rs", &f)
            .description
            .expect("description");
        assert!(!body.contains("Suggested fix"), "{body}");
    }

    #[test]
    fn long_summaries_are_cut() {
        let long = "x".repeat(MAX_TITLE_CHARS + 10);
        let opts = task_options("c", "a.rs", &finding(Some(&long), None));
        let summary = opts.title.trim_start_matches("CodeRabbit major: ");
        assert_eq!(summary.chars().count(), MAX_TITLE_CHARS + 1);
        assert!(summary.ends_with('…'));
    }
}
