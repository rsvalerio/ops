//! `JUnit` XML report for [`super::ProgressDisplay`] (TASK-2338).
//!
//! `--junit <file>` writes one `<testcase>` per plan step when the run
//! finishes, so CI test-report consumers (dorny/test-reporter, GitLab's
//! `artifacts:reports:junit`, Jenkins, …) can show which `ops verify` / `qa`
//! steps failed without scraping the log. A failed step carries a
//! `<failure>` whose `message` is the runner's failure message and whose body
//! is the tail of the step's captured output; a step that never ran is
//! `<skipped/>`.
//!
//! Step output is untrusted text and routinely carries ANSI escapes, which are
//! not legal XML 1.0 characters. Every value is escaped, and characters XML
//! cannot represent are replaced with U+FFFD, so a hostile or merely colourful
//! step cannot produce a report that consumers reject.

use super::github::StepOutcome;
use ops_core::output::StepStatus;
use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::path::PathBuf;

/// Output lines kept per step for a failure body.
const FAILURE_TAIL_LINES: usize = 50;

/// One finished (or never-run) step.
struct Case {
    id: String,
    display: String,
    status: StepStatus,
    duration_secs: f64,
    failure: Option<Failure>,
}

struct Failure {
    message: String,
    tail: Vec<Box<str>>,
}

/// Collects step results while a plan runs and writes the `JUnit` report when
/// it finishes.
pub(super) struct JunitReport {
    path: PathBuf,
    output: HashMap<String, VecDeque<Box<str>>>,
    cases: Vec<Case>,
}

impl JunitReport {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            output: HashMap::new(),
            cases: Vec::new(),
        }
    }

    /// Drop everything collected for a previous plan.
    pub(super) fn reset(&mut self) {
        self.output.clear();
        self.cases.clear();
    }

    /// Retain one output line (stdout or stderr) of step `id`, keeping only
    /// the most recent [`FAILURE_TAIL_LINES`].
    pub(super) fn record_output(&mut self, id: &str, line: &str) {
        let ring = self.output.entry(id.to_string()).or_default();
        if ring.len() == FAILURE_TAIL_LINES {
            ring.pop_front();
        }
        ring.push_back(line.into());
    }

    /// Record a step that reached a terminal state; `outcome.failure` is
    /// the runner's message, `Some` only for a failed step.
    pub(super) fn step_finished(&mut self, display: &str, outcome: &StepOutcome<'_>) {
        let tail = self.output.remove(outcome.id).unwrap_or_default();
        self.cases.push(Case {
            id: outcome.id.to_string(),
            display: display.to_string(),
            status: outcome.status,
            duration_secs: outcome.duration_secs,
            failure: outcome.failure.map(|message| Failure {
                message: message.to_string(),
                tail: tail.into(),
            }),
        });
    }

    /// Render the report as a `JUnit` XML document.
    fn render(&self, duration_secs: f64) -> String {
        let failures = self
            .cases
            .iter()
            .filter(|c| matches!(c.status, StepStatus::Failed))
            .count();
        let skipped = self
            .cases
            .iter()
            .filter(|c| matches!(c.status, StepStatus::Skipped))
            .count();
        let tests = self.cases.len();
        let time = format!("{duration_secs:.3}");
        let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        // `write!` into a `String` cannot fail.
        let _ = writeln!(
            out,
            "<testsuites name=\"ops\" tests=\"{tests}\" failures=\"{failures}\" \
             errors=\"0\" skipped=\"{skipped}\" time=\"{time}\">"
        );
        let _ = writeln!(
            out,
            "  <testsuite name=\"ops\" tests=\"{tests}\" failures=\"{failures}\" \
             errors=\"0\" skipped=\"{skipped}\" time=\"{time}\">"
        );
        for case in &self.cases {
            let _ = write!(
                out,
                "    <testcase name=\"{}\" classname=\"ops.{}\" time=\"{:.3}\"",
                escape_xml(&case.display),
                escape_xml(&case.id),
                case.duration_secs
            );
            if let Some(ref failure) = case.failure {
                let _ = write!(
                    out,
                    ">\n      <failure message=\"{}\" type=\"failure\">",
                    escape_xml(&failure.message)
                );
                let mut body = failure.message.clone();
                for line in &failure.tail {
                    body.push('\n');
                    body.push_str(line);
                }
                out.push_str(&escape_xml(&body));
                out.push_str("</failure>\n    </testcase>\n");
            } else if matches!(case.status, StepStatus::Skipped) {
                out.push_str(">\n      <skipped/>\n    </testcase>\n");
            } else {
                out.push_str("/>\n");
            }
        }
        out.push_str("  </testsuite>\n</testsuites>\n");
        out
    }

    /// Write the report to the requested path, replacing any previous file.
    ///
    /// # Errors
    ///
    /// The underlying write error; the caller reports it without failing the
    /// run the report describes.
    pub(super) fn write(&self, duration_secs: f64) -> std::io::Result<()> {
        std::fs::write(&self.path, self.render(duration_secs))
    }

    pub(super) const fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// Escape `s` for use in XML text or a double-quoted attribute. Characters
/// XML 1.0 cannot carry at all (C0 controls other than tab/LF/CR, such as the
/// ESC that starts an ANSI colour sequence) become U+FFFD. CR and LF are
/// written as character references so attribute values keep them instead of
/// having them normalised to spaces.
fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push('\t'),
            c if c < ' ' || matches!(c, '\u{FFFE}' | '\u{FFFF}') => out.push('\u{FFFD}'),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(
        id: &'static str,
        status: StepStatus,
        duration_secs: f64,
        failure: Option<&'static str>,
    ) -> StepOutcome<'static> {
        StepOutcome {
            id,
            status,
            duration_secs,
            failure,
        }
    }

    #[test]
    fn escape_covers_markup_and_illegal_chars() {
        assert_eq!(
            escape_xml("a<b>&\"c'\n\x1b[31mred\t"),
            "a&lt;b&gt;&amp;&quot;c&apos;&#10;\u{FFFD}[31mred\t"
        );
    }

    #[test]
    fn one_testcase_per_step_with_failure_message_and_tail() {
        let mut report = JunitReport::new(PathBuf::from("unused.xml"));
        report.record_output("build", "compiling");
        report.step_finished(
            "cargo build",
            &outcome("build", StepStatus::Succeeded, 1.5, None),
        );
        for i in 0..(FAILURE_TAIL_LINES + 5) {
            report.record_output("test", &format!("line {i}"));
        }
        report.step_finished(
            "cargo test",
            &outcome("test", StepStatus::Failed, 2.25, Some("exit status: 101")),
        );
        report.step_finished("cargo doc", &outcome("doc", StepStatus::Skipped, 0.0, None));

        let xml = report.render(3.75);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"));
        assert!(xml.contains(
            "<testsuite name=\"ops\" tests=\"3\" failures=\"1\" errors=\"0\" skipped=\"1\" time=\"3.750\">"
        ));
        assert!(
            xml.contains("<testcase name=\"cargo build\" classname=\"ops.build\" time=\"1.500\"/>")
        );
        assert!(xml.contains("<failure message=\"exit status: 101\" type=\"failure\">"));
        assert!(xml.contains(&format!("line {}", FAILURE_TAIL_LINES + 4)));
        assert!(xml.contains("line 5&#10;"));
        assert!(!xml.contains("line 4&#10;"), "tail must be bounded: {xml}");
        assert!(xml.contains(
            "<testcase name=\"cargo doc\" classname=\"ops.doc\" time=\"0.000\">\n      <skipped/>"
        ));
        assert_eq!(xml.matches("<testcase ").count(), 3);
    }

    #[test]
    fn successful_step_output_is_not_carried_into_a_later_failure() {
        let mut report = JunitReport::new(PathBuf::from("unused.xml"));
        report.record_output("a", "from a");
        report.step_finished("a", &outcome("a", StepStatus::Succeeded, 0.0, None));
        report.step_finished("b", &outcome("b", StepStatus::Failed, 0.0, Some("boom")));
        let xml = report.render(0.0);
        assert!(!xml.contains("from a"));
    }

    #[test]
    fn write_replaces_the_report_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("junit.xml");
        std::fs::write(&path, "stale").expect("seed");
        let mut report = JunitReport::new(path.clone());
        report.step_finished("a", &outcome("a", StepStatus::Succeeded, 0.5, None));
        report.write(0.5).expect("write report");
        let written = std::fs::read_to_string(&path).expect("read report");
        assert!(!written.contains("stale"));
        assert!(written.contains("tests=\"1\""));
    }
}
