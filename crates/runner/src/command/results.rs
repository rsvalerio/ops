//! Result types for command execution.

use ops_core::config::CommandId;
use std::time::Duration;

/// Result of running a single command.
#[derive(Debug, Clone)]
pub struct StepResult {
    pub id: CommandId,
    pub success: bool,
    pub duration: Duration,
    pub stdout: String,
    pub stderr: String,
    pub message: Option<String>,
}

impl StepResult {
    /// DUP-003: Private helper to construct base StepResult with common defaults.
    fn new(id: impl Into<CommandId>, success: bool, duration: Duration) -> Self {
        Self {
            id: id.into(),
            success,
            duration,
            stdout: String::new(),
            stderr: String::new(),
            message: None,
        }
    }

    /// Construct a failure result for an IO/timeout error (no captured output).
    pub fn failure(id: impl Into<CommandId>, duration: Duration, message: String) -> Self {
        Self {
            message: Some(message),
            ..Self::new(id, false, duration)
        }
    }

    /// Construct a result for a skipped command (abort flag set).
    pub fn skipped(id: impl Into<CommandId>) -> Self {
        Self::new(id, true, Duration::ZERO)
    }

    #[cfg(test)]
    pub fn success(id: &str, duration: Duration) -> Self {
        Self::new(id, true, duration)
    }
}

/// Captured output from a command execution.
/// Note: Invalid UTF-8 bytes are replaced with U+FFFD replacement character.
/// This is acceptable for CLI display but loses original byte sequences.
///
/// EFF-004: We use `into_owned()` on `Cow<str>` to convert to `String`. This is
/// necessary because we need owned strings for the `StepResult` struct which may
/// outlive the original `std::process::Output`. For most CLI outputs (which are
/// typically small), this overhead is negligible. If processing very large outputs
/// (e.g., >1MB), consider streaming output directly instead of buffering.
#[derive(Debug)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub status_message: String,
}

impl CommandOutput {
    pub fn from_raw(output: std::process::Output) -> Self {
        let success = output.status.success();
        Self {
            success,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            status_message: output.status.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::test_utils::make_test_output;

    #[test]
    fn command_output_from_raw_success() {
        let output = make_test_output(0, b"hello world", b"");
        let cmd_output = CommandOutput::from_raw(output);
        assert!(cmd_output.success);
        assert_eq!(cmd_output.stdout, "hello world");
        assert!(cmd_output.stderr.is_empty());
    }

    #[test]
    fn command_output_from_raw_failure() {
        let output = make_test_output(1, b"", b"error message");
        let cmd_output = CommandOutput::from_raw(output);
        assert!(!cmd_output.success);
        assert!(cmd_output.stdout.is_empty());
        assert_eq!(cmd_output.stderr, "error message");
    }

    #[test]
    fn command_output_from_raw_invalid_utf8() {
        let output = make_test_output(0, b"hello\xffworld", b"test\xfe");
        let cmd_output = CommandOutput::from_raw(output);
        assert!(cmd_output.stdout.contains("hello"));
        assert!(cmd_output.stdout.contains("world"));
        assert!(cmd_output.stderr.contains("test"));
    }

    #[test]
    fn command_output_status_message() {
        let output = make_test_output(0, b"", b"");
        let cmd_output = CommandOutput::from_raw(output);
        assert!(!cmd_output.status_message.is_empty());
    }

    /// TQ-009: StepResult clone produces equal copy.
    #[test]
    fn step_result_clone_produces_equal_copy() {
        let original = StepResult {
            id: "test".into(),
            success: true,
            duration: Duration::from_secs(5),
            stdout: "output".to_string(),
            stderr: "err".to_string(),
            message: Some("msg".to_string()),
        };
        let cloned = original.clone();
        assert_eq!(cloned.id, original.id);
        assert_eq!(cloned.success, original.success);
        assert_eq!(cloned.duration, original.duration);
        assert_eq!(cloned.stdout, original.stdout);
        assert_eq!(cloned.stderr, original.stderr);
        assert_eq!(cloned.message, original.message);
    }

    /// TQ-009: StepResult Debug output includes key fields.
    #[test]
    fn step_result_debug_includes_fields() {
        let result = StepResult::failure("debug_cmd", Duration::from_millis(42), "oops".into());
        let debug = format!("{:?}", result);
        assert!(debug.contains("debug_cmd"), "Debug should include id");
        assert!(debug.contains("oops"), "Debug should include message");
        assert!(
            debug.contains("false"),
            "Debug should include success=false"
        );
    }

    #[test]
    fn step_result_failure_constructor() {
        let duration = Duration::from_millis(100);
        let result = StepResult::failure("test_cmd", duration, "error occurred".to_string());
        assert_eq!(result.id, "test_cmd");
        assert!(!result.success);
        assert_eq!(result.duration, duration);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.is_empty());
        assert_eq!(result.message, Some("error occurred".to_string()));
    }
}
