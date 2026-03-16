//! Events emitted during command execution for plain-text (theme) output.

use ops_core::config::CommandId;
use serde::Serialize;

/// Events emitted during command execution for plain-text (theme) output.
#[derive(Debug, Clone, Serialize)]
pub enum RunnerEvent {
    /// Execution plan started (list of command ids).
    PlanStarted { command_ids: Vec<CommandId> },
    /// A single command started.
    StepStarted {
        id: CommandId,
        /// Display string for the command (e.g. "cargo build --all-targets").
        display_cmd: Option<String>,
    },
    /// A single command produced stdout/stderr line(s).
    StepOutput {
        id: CommandId,
        line: String,
        stderr: bool,
    },
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
            command_ids: vec!["build".to_string(), "test".to_string()],
        };
        let json = serde_json::to_string(&event).expect("should serialize");
        assert!(json.contains("PlanStarted"));
        assert!(json.contains("build"));
        assert!(json.contains("test"));
    }

    #[test]
    fn step_finished_serializes_with_duration() {
        let event = RunnerEvent::StepFinished {
            id: "cargo build".to_string(),
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
            id: "test".to_string(),
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
            id: "build".to_string(),
            line: "warning: unused variable".to_string(),
            stderr: true,
        };
        let json = serde_json::to_string(&event).expect("should serialize");
        assert!(json.contains("StepOutput"));
        assert!(json.contains("stderr"));
    }

    #[test]
    fn step_skipped_serializes() {
        let event = RunnerEvent::StepSkipped {
            id: "lint".to_string(),
            display_cmd: Some("cargo clippy".to_string()),
        };
        let json = serde_json::to_string(&event).expect("should serialize");
        assert!(json.contains("StepSkipped"));
    }
}
