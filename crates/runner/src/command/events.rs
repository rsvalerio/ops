//! Events emitted during command execution for plain-text (theme) output,
//! plus the [`PlanLifecycle`] bookend that emits PlanStarted / RunFinished
//! around every plan run.

use ops_core::config::CommandId;
use serde::{Serialize, Serializer};
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

/// A captured stdout/stderr line carried by [`RunnerEvent::StepOutput`].
///
/// PERF-3 / TASK-0732: holds an `Arc<str>` view onto the parent capture
/// buffer plus the byte range of this line. `emit_output_events` constructs
/// one `Arc<str>` per buffer (transferring ownership of the existing
/// `String` alloc — no copy) and emits per-line `OutputLine` values that
/// share the buffer via cheap atomic refcount increments. A noisy step that
/// previously paid one heap allocation per line (`line.to_string()`) now
/// pays one per buffer.
///
/// JSON serialization preserves the historical shape: the field renders as
/// a plain string, identical to the pre-fix `line: String` form.
#[derive(Clone)]
pub struct OutputLine {
    buf: Arc<str>,
    range: Range<usize>,
}

impl OutputLine {
    /// Build an `OutputLine` over the entire `Arc<str>` buffer.
    pub fn whole(buf: Arc<str>) -> Self {
        let len = buf.len();
        Self { buf, range: 0..len }
    }

    /// Build an `OutputLine` over a sub-range of `buf`.
    ///
    /// Caller is responsible for `range` being a valid byte slice that lands
    /// on UTF-8 boundaries (the normal case when `range` came from
    /// `str::lines` / split-on-newline byte indexing).
    pub fn slice(buf: Arc<str>, range: Range<usize>) -> Self {
        debug_assert!(range.end <= buf.len());
        debug_assert!(buf.is_char_boundary(range.start));
        debug_assert!(buf.is_char_boundary(range.end));
        Self { buf, range }
    }

    /// Visible bytes of this line.
    pub fn as_str(&self) -> &str {
        &self.buf[self.range.clone()]
    }

    /// PERF-3 / TASK-0838: crate-internal handle on the backing buffer so
    /// regression tests can pin the Arc-sharing model with `Arc::ptr_eq` /
    /// `Arc::strong_count`. Not part of the public API — the buffer
    /// representation is an implementation detail of how per-line events
    /// avoid per-line allocations.
    #[cfg(test)]
    pub(crate) fn buf_arc(&self) -> &Arc<str> {
        &self.buf
    }
}

impl std::fmt::Debug for OutputLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl std::fmt::Display for OutputLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::ops::Deref for OutputLine {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl Serialize for OutputLine {
    /// Preserve the pre-fix JSON shape: the field renders as a plain string.
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.as_str())
    }
}

impl PartialEq for OutputLine {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for OutputLine {}

impl From<&str> for OutputLine {
    fn from(s: &str) -> Self {
        Self::whole(Arc::from(s))
    }
}

impl From<String> for OutputLine {
    fn from(s: String) -> Self {
        Self::whole(Arc::from(s))
    }
}

/// Tracks the lifecycle of a plan execution (PlanStarted → RunFinished bookends).
pub(crate) struct PlanLifecycle {
    start: Instant,
}

impl PlanLifecycle {
    pub(crate) fn begin(command_ids: &[CommandId], on_event: &mut impl FnMut(RunnerEvent)) -> Self {
        on_event(RunnerEvent::PlanStarted {
            command_ids: command_ids.to_vec(),
        });
        Self {
            start: Instant::now(),
        }
    }

    /// FN-9 / TASK-0197+0211: take `success` explicitly rather than a full
    /// `&[StepResult]`. Callers already walk the results inside the run loop
    /// to compute success anyway, so threading a bool is clearer than
    /// handing over the entire slice for an `iter().all()` re-walk. It also
    /// prevents a future refactor from passing a partial-result slice and
    /// silently misreporting the run outcome.
    pub(crate) fn finish(self, success: bool, on_event: &mut impl FnMut(RunnerEvent)) {
        on_event(RunnerEvent::RunFinished {
            duration_secs: self.start.elapsed().as_secs_f64(),
            success,
        });
    }
}

/// Events emitted during command execution for plain-text (theme) output.
///
/// API-9 / TASK-0455: marked `#[non_exhaustive]` so adding new variants
/// (e.g. `StepOutputDropped` from TASK-0457) is not a SemVer break for
/// downstream matchers in display / CLI / extensions. Cross-crate `match`
/// sites must include a wildcard arm.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub enum RunnerEvent {
    /// Execution plan started (list of command ids).
    PlanStarted { command_ids: Vec<CommandId> },
    /// A single command started.
    ///
    /// OWN-3 / TASK-0770: `display_cmd` is kept as `Option<String>` rather
    /// than `Option<Arc<str>>` intentionally. The Started/Finished pair
    /// owns a separate snapshot per event so each variant is independently
    /// movable into the bounded mpsc channel without lifetime coupling, and
    /// the public `RunnerEvent` serde shape stays a plain string for
    /// downstream JSON consumers (see AC #3 on the task). The single extra
    /// allocation per spawn is below the spawn cost itself and not worth
    /// the API / test churn an `Arc<str>` payload would force.
    StepStarted {
        id: CommandId,
        /// Display string for the command (e.g. "cargo build --all-targets").
        display_cmd: Option<String>,
    },
    /// A single command produced stdout/stderr line(s).
    ///
    /// PERF-3 / TASK-0732: `line` is an [`OutputLine`] view sharing one
    /// `Arc<str>` per capture buffer. Pre-fix this was `String`, paying one
    /// heap allocation per line; the new shape preserves the JSON
    /// serialization (the field still renders as a plain string).
    StepOutput {
        id: CommandId,
        line: OutputLine,
        stderr: bool,
    },
    /// CONC-7 / TASK-0457: emitted when the per-task event buffer
    /// overflowed during a noisy command, so the display can surface
    /// "(N output lines dropped under load)" instead of silently losing
    /// stdout/stderr lines that explain the failure.
    StepOutputDropped { id: CommandId, dropped_count: u64 },
    /// A single command finished successfully.
    StepFinished {
        id: CommandId,
        duration_secs: f64,
        display_cmd: Option<String>,
    },
    /// A single command was skipped (e.g. abort flag set before execution).
    StepSkipped {
        id: CommandId,
        display_cmd: Option<String>,
    },
    /// A single command failed.
    StepFailed {
        id: CommandId,
        duration_secs: f64,
        message: String,
        display_cmd: Option<String>,
    },
    /// Entire run finished (total duration, success).
    RunFinished { duration_secs: f64, success: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runner_event_serializes_to_json() {
        let event = RunnerEvent::PlanStarted {
            command_ids: vec!["build".into(), "test".into()],
        };
        let json = serde_json::to_string(&event).expect("should serialize");
        assert!(json.contains("PlanStarted"));
        assert!(json.contains("build"));
        assert!(json.contains("test"));
    }

    #[test]
    fn step_finished_serializes_with_duration() {
        let event = RunnerEvent::StepFinished {
            id: "cargo build".into(),
            duration_secs: 1.234,
            display_cmd: Some("cargo build --release".to_string()),
        };
        let json = serde_json::to_string(&event).expect("should serialize");
        assert!(json.contains("StepFinished"));
        assert!(json.contains("1.234"));
    }

    #[test]
    fn step_failed_serializes_with_message() {
        let event = RunnerEvent::StepFailed {
            id: "test".into(),
            duration_secs: 0.5,
            message: "exit status: 101".to_string(),
            display_cmd: None,
        };
        let json = serde_json::to_string(&event).expect("should serialize");
        assert!(json.contains("StepFailed"));
        assert!(json.contains("exit status: 101"));
    }

    #[test]
    fn run_finished_serializes_success_flag() {
        let event_success = RunnerEvent::RunFinished {
            duration_secs: 5.0,
            success: true,
        };
        let event_failure = RunnerEvent::RunFinished {
            duration_secs: 2.0,
            success: false,
        };
        let json_success = serde_json::to_string(&event_success).expect("should serialize");
        let json_failure = serde_json::to_string(&event_failure).expect("should serialize");
        assert!(json_success.contains("true"));
        assert!(json_failure.contains("false"));
    }

    #[test]
    fn step_output_serializes_stderr_flag() {
        let event = RunnerEvent::StepOutput {
            id: "build".into(),
            line: OutputLine::from("warning: unused variable"),
            stderr: true,
        };
        let json = serde_json::to_string(&event).expect("should serialize");
        assert!(json.contains("StepOutput"));
        assert!(json.contains("stderr"));
    }

    #[test]
    fn step_skipped_serializes() {
        let event = RunnerEvent::StepSkipped {
            id: "lint".into(),
            display_cmd: Some("cargo clippy".to_string()),
        };
        let json = serde_json::to_string(&event).expect("should serialize");
        assert!(json.contains("StepSkipped"));
    }
}
