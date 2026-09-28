//! GitHub Actions integration for [`super::ProgressDisplay`] (TASK-2325).
//!
//! When `ops` runs under GitHub Actions (`GITHUB_ACTIONS=true`) the plain
//! step lines are hard to read in the job log: a failing step only shows its
//! stderr tail, and nothing is collapsible. This module adds the three
//! workflow-command surfaces the runner understands:
//!
//! - a collapsible `::group::` per finished step holding that step's captured
//!   output,
//! - an `::error` annotation per failed step naming the command and carrying
//!   its output tail,
//! - a Markdown run summary appended to `$GITHUB_STEP_SUMMARY`.
//!
//! Step output is untrusted text: a line such as `::add-mask::…` or
//! `::stop-commands::…` would otherwise be interpreted by the runner as a
//! workflow command that `ops` itself emitted. Group bodies are therefore
//! fenced with `::stop-commands::<token>` / `::<token>::`, the token being an
//! unguessable per-run value, and every value placed inside a workflow
//! command is escaped the way `@actions/core` escapes it.

use ops_core::output::StepStatus;
use ops_theme as theme;
use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::PathBuf;

/// Most recent output lines retained per step for its group body. The runner
/// already caps each captured stream at `OPS_OUTPUT_BYTE_CAP`; this bounds
/// the per-step line count the display holds until the step finishes.
const MAX_GROUP_LINES: usize = 1000;

/// Output lines carried in a failure annotation.
const ANNOTATION_TAIL_LINES: usize = 10;

/// Per-step output ring plus the number of older lines it evicted.
#[derive(Default)]
struct StepOutput {
    lines: VecDeque<Box<str>>,
    omitted: usize,
}

/// One row of the step summary table.
struct SummaryRow {
    display: String,
    status: StepStatus,
    duration_secs: f64,
}

/// How a step ended, as handed to [`GithubActions::step_finished`].
pub(super) struct StepOutcome<'a> {
    pub id: &'a str,
    pub status: StepStatus,
    pub duration_secs: f64,
    /// Failure message; `Some` only for a failed step.
    pub failure: Option<&'a str>,
}

/// GitHub Actions reporter: collects per-step output while a plan runs and
/// renders workflow commands / the step summary when steps finish.
pub(super) struct GithubActions {
    summary_path: Option<PathBuf>,
    stop_token: String,
    output: HashMap<String, StepOutput>,
    rows: Vec<SummaryRow>,
}

impl GithubActions {
    /// Enabled only when the GitHub Actions runner says so; the summary path
    /// is taken from `GITHUB_STEP_SUMMARY` when set and non-empty.
    pub(super) fn from_env() -> Option<Self> {
        if std::env::var("GITHUB_ACTIONS").ok().as_deref() != Some("true") {
            return None;
        }
        let summary_path = std::env::var_os("GITHUB_STEP_SUMMARY")
            .filter(|p| !p.is_empty())
            .map(PathBuf::from);
        Some(Self::new(summary_path))
    }

    pub(super) fn new(summary_path: Option<PathBuf>) -> Self {
        Self {
            summary_path,
            stop_token: stop_token(),
            output: HashMap::new(),
            rows: Vec::new(),
        }
    }

    /// Drop everything collected for a previous plan.
    pub(super) fn reset(&mut self) {
        self.output.clear();
        self.rows.clear();
    }

    /// Retain one output line (stdout or stderr) of step `id`.
    pub(super) fn record_output(&mut self, id: &str, line: &str) {
        let entry = self.output.entry(id.to_string()).or_default();
        if entry.lines.len() == MAX_GROUP_LINES {
            entry.lines.pop_front();
            entry.omitted = entry.omitted.saturating_add(1);
        }
        entry.lines.push_back(line.into());
    }

    /// Record a step that was never run (orphaned by cancellation). It gets a
    /// summary row but no group, since it produced no output.
    pub(super) fn record_skipped(&mut self, display: &str, duration_secs: f64) {
        self.rows.push(SummaryRow {
            display: display.to_string(),
            status: StepStatus::Skipped,
            duration_secs,
        });
    }

    /// Render the workflow-command lines for a finished step: its output
    /// group and, when `failure` is set, an `::error` annotation with the
    /// failure message and the output tail.
    pub(super) fn step_finished(
        &mut self,
        display: &str,
        outcome: &StepOutcome<'_>,
    ) -> Vec<String> {
        let &StepOutcome {
            id,
            status,
            duration_secs,
            failure,
        } = outcome;
        self.rows.push(SummaryRow {
            display: display.to_string(),
            status,
            duration_secs,
        });
        let output = self.output.remove(id).unwrap_or_default();

        let title = format!(
            "{display} ({} in {})",
            status_word(status),
            theme::format_duration(duration_secs)
        );
        let mut lines = vec![format!("::group::{}", escape_data(&title))];
        if output.omitted > 0 {
            lines.push(format!("[ops] {} earlier line(s) omitted", output.omitted));
        }
        if !output.lines.is_empty() {
            lines.push(format!("::stop-commands::{}", self.stop_token));
            lines.extend(output.lines.iter().map(ToString::to_string));
            lines.push(format!("::{}::", self.stop_token));
        }
        lines.push("::endgroup::".to_string());

        if let Some(message) = failure {
            let mut body = format!("{display} failed: {message}");
            let skip = output.lines.len().saturating_sub(ANNOTATION_TAIL_LINES);
            for line in output.lines.iter().skip(skip) {
                body.push('\n');
                body.push_str(line);
            }
            lines.push(format!(
                "::error title={}::{}",
                escape_property(&format!("ops: {display} failed")),
                escape_data(&body)
            ));
        }
        lines
    }

    /// Render the Markdown run summary.
    fn render_summary(&self, success: bool, duration_secs: f64) -> String {
        let failed = self
            .rows
            .iter()
            .filter(|r| matches!(r.status, StepStatus::Failed))
            .count();
        let succeeded = self
            .rows
            .iter()
            .filter(|r| matches!(r.status, StepStatus::Succeeded))
            .count();
        let elapsed = theme::format_duration(duration_secs);
        let mut out = String::new();
        let headline = if success {
            format!(
                "### ops: {succeeded}/{} step(s) succeeded in {elapsed}",
                self.rows.len()
            )
        } else {
            format!(
                "### ops: failed ({failed} failed, {succeeded} succeeded of {}) in {elapsed}",
                self.rows.len()
            )
        };
        // `write!` into a `String` cannot fail.
        let _ = writeln!(out, "{headline}\n");
        if !self.rows.is_empty() {
            out.push_str("| Step | Status | Duration |\n|---|---|---|\n");
            for row in &self.rows {
                let _ = writeln!(
                    out,
                    "| {} | {} | {} |",
                    escape_markdown_cell(&row.display),
                    status_word(row.status),
                    theme::format_duration(row.duration_secs)
                );
            }
        }
        out.push('\n');
        out
    }

    /// Append the run summary to `$GITHUB_STEP_SUMMARY`. A write failure is
    /// logged and otherwise ignored: the summary is a convenience and must
    /// never fail the run it describes.
    pub(super) fn write_summary(&self, success: bool, duration_secs: f64) {
        let Some(ref path) = self.summary_path else {
            return;
        };
        let summary = self.render_summary(success, duration_secs);
        let result = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| f.write_all(summary.as_bytes()));
        if let Err(e) = result {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to write GitHub Actions step summary"
            );
        }
    }
}

const fn status_word(status: StepStatus) -> &'static str {
    match status {
        StepStatus::Succeeded => "succeeded",
        StepStatus::Failed => "failed",
        StepStatus::Skipped => "skipped",
        StepStatus::Running => "running",
        // `StepStatus` is `#[non_exhaustive]`; Pending and future variants.
        _ => "pending",
    }
}

/// Unguessable token for `::stop-commands::`. `RandomState` is seeded from
/// OS randomness once per process, so the token cannot be predicted by the
/// step output it fences.
fn stop_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u32(std::process::id());
    format!("ops-{:016x}", hasher.finish())
}

/// Escape a workflow-command message, as `@actions/core`'s `escapeData`.
fn escape_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escape a workflow-command property value, as `@actions/core`'s
/// `escapeProperty`.
fn escape_property(s: &str) -> String {
    escape_data(s).replace(':', "%3A").replace(',', "%2C")
}

/// Keep a step label on one Markdown table row: newlines would end the row
/// and a bare `|` would split the cell.
fn escape_markdown_cell(s: &str) -> String {
    s.replace(['\r', '\n'], " ").replace('|', "\\|")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_match_actions_toolkit() {
        assert_eq!(escape_data("a%b\r\nc"), "a%25b%0D%0Ac");
        assert_eq!(escape_property("a:b,c\n"), "a%3Ab%2Cc%0A");
        assert_eq!(escape_markdown_cell("a|b\nc"), "a\\|b c");
    }

    #[test]
    fn successful_step_is_a_group_fencing_its_output() {
        let mut gha = GithubActions::new(None);
        gha.record_output("build", "compiling");
        gha.record_output("build", "::add-mask::secret");
        let lines = gha.step_finished(
            "cargo build",
            &StepOutcome {
                id: "build",
                status: StepStatus::Succeeded,
                duration_secs: 1.5,
                failure: None,
            },
        );
        let token = gha.stop_token.clone();
        assert_eq!(
            lines,
            vec![
                "::group::cargo build (succeeded in 1.50s)".to_string(),
                format!("::stop-commands::{token}"),
                "compiling".to_string(),
                "::add-mask::secret".to_string(),
                format!("::{token}::"),
                "::endgroup::".to_string(),
            ]
        );
        assert!(!lines.iter().any(|l| l.starts_with("::error")));
    }

    #[test]
    fn step_without_output_is_still_a_group() {
        let mut gha = GithubActions::new(None);
        let lines = gha.step_finished(
            "cargo fmt",
            &StepOutcome {
                id: "fmt",
                status: StepStatus::Succeeded,
                duration_secs: 0.1,
                failure: None,
            },
        );
        assert_eq!(
            lines,
            vec![
                "::group::cargo fmt (succeeded in 0.10s)".to_string(),
                "::endgroup::".to_string()
            ]
        );
    }

    #[test]
    fn failed_step_is_annotated_with_command_and_output_tail() {
        let mut gha = GithubActions::new(None);
        for i in 0..15 {
            gha.record_output("test", &format!("line {i}"));
        }
        let lines = gha.step_finished(
            "cargo test",
            &StepOutcome {
                id: "test",
                status: StepStatus::Failed,
                duration_secs: 2.0,
                failure: Some("exit status: 101"),
            },
        );
        let annotation = lines.last().expect("annotation line");
        assert!(
            annotation.starts_with(
                "::error title=ops%3A cargo test failed::cargo test failed: exit status: 101"
            ),
            "got: {annotation}"
        );
        assert!(annotation.contains("%0Aline 14"));
        assert!(annotation.contains("%0Aline 5"));
        assert!(!annotation.contains("line 4%0A"), "tail must be bounded");
        assert!(lines.contains(&"::endgroup::".to_string()));
    }

    #[test]
    fn output_ring_is_bounded() {
        let mut gha = GithubActions::new(None);
        for i in 0..(MAX_GROUP_LINES + 3) {
            gha.record_output("x", &i.to_string());
        }
        let lines = gha.step_finished(
            "x",
            &StepOutcome {
                id: "x",
                status: StepStatus::Succeeded,
                duration_secs: 0.0,
                failure: None,
            },
        );
        assert!(lines.contains(&"[ops] 3 earlier line(s) omitted".to_string()));
        assert!(!lines.contains(&"2".to_string()));
        assert!(lines.contains(&"3".to_string()));
    }

    #[test]
    fn summary_is_appended_to_step_summary_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("summary.md");
        std::fs::write(&path, "previous\n").expect("seed summary");
        let mut gha = GithubActions::new(Some(path.clone()));
        gha.step_finished(
            "cargo build",
            &StepOutcome {
                id: "a",
                status: StepStatus::Succeeded,
                duration_secs: 1.0,
                failure: None,
            },
        );
        gha.step_finished(
            "cargo t|est",
            &StepOutcome {
                id: "b",
                status: StepStatus::Failed,
                duration_secs: 2.0,
                failure: Some("boom"),
            },
        );
        gha.record_skipped("cargo doc", 0.0);
        gha.write_summary(false, 3.0);
        let written = std::fs::read_to_string(&path).expect("read summary");
        assert!(written.starts_with("previous\n"), "must append: {written}");
        assert!(written.contains("### ops: failed (1 failed, 1 succeeded of 3)"));
        assert!(written.contains("| cargo build | succeeded | 1.00s |"));
        assert!(written.contains("| cargo t\\|est | failed | 2.00s |"));
        assert!(written.contains("| cargo doc | skipped |"));
    }

    #[test]
    fn summary_without_path_is_a_no_op() {
        let gha = GithubActions::new(None);
        gha.write_summary(true, 1.0);
    }
}
