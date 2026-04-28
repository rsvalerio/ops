//! Tests for build_command and StepResult construction.

use super::*;

#[test]
fn step_result_failure_creates_correct_result() {
    let duration = Duration::from_millis(100);
    let result = StepResult::failure("test_cmd", duration, "test error".to_string());
    assert_eq!(result.id, "test_cmd");
    assert!(!result.success);
    assert_eq!(result.duration, duration);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    assert_eq!(result.message, Some("test error".to_string()));
}

#[test]
fn build_command_sets_program_and_args() {
    let spec = exec_spec("cargo", &["build", "--release"]);
    let cmd = build_command(&spec, std::path::Path::new("."), &test_vars()).unwrap();
    assert_eq!(cmd.as_std().get_program(), "cargo");
    let args: Vec<_> = cmd.as_std().get_args().collect();
    assert_eq!(args, vec!["build", "--release"]);
}

#[test]
fn build_command_uses_spec_cwd_when_provided() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut spec = exec_spec("echo", &["test"]);
    spec.cwd = Some(temp_dir.path().to_path_buf());
    let cmd = build_command(&spec, std::path::Path::new("."), &test_vars()).unwrap();
    assert_eq!(cmd.as_std().get_current_dir(), Some(temp_dir.path()));
}

/// TQ-005: Tests for build_command error paths.
mod build_command_error_tests {
    use super::*;

    #[test]
    fn build_command_with_nonexistent_cwd_still_builds() {
        let mut spec = exec_spec("echo", &["test"]);
        spec.cwd = Some(PathBuf::from("/nonexistent/path/that/does/not/exist"));
        let cmd = build_command(&spec, std::path::Path::new("."), &test_vars()).unwrap();
        assert_eq!(cmd.as_std().get_program(), "echo");
    }

    #[test]
    fn build_command_with_relative_cwd() {
        let mut spec = exec_spec("echo", &["test"]);
        spec.cwd = Some(PathBuf::from("relative/path"));
        let cmd = build_command(&spec, std::path::Path::new("/base"), &test_vars()).unwrap();
        let current_dir = cmd.as_std().get_current_dir();
        assert_eq!(
            current_dir,
            Some(std::path::Path::new("/base/relative/path"))
        );
    }

    #[test]
    fn build_command_with_absolute_cwd() {
        let mut spec = exec_spec("echo", &["test"]);
        spec.cwd = Some(PathBuf::from("/absolute/path"));
        let cmd = build_command(&spec, std::path::Path::new("/base"), &test_vars()).unwrap();
        let current_dir = cmd.as_std().get_current_dir();
        assert_eq!(current_dir, Some(std::path::Path::new("/absolute/path")));
    }

    #[test]
    fn build_command_with_empty_args() {
        let spec = exec_spec("echo", &[]);
        let cmd = build_command(&spec, std::path::Path::new("."), &test_vars()).unwrap();
        assert_eq!(cmd.as_std().get_program(), "echo");
        let args: Vec<_> = cmd.as_std().get_args().collect();
        assert!(args.is_empty());
    }

    #[test]
    fn build_command_with_many_args() {
        let spec = exec_spec("echo", &["a", "b", "c", "d", "e"]);
        let cmd = build_command(&spec, std::path::Path::new("."), &test_vars()).unwrap();
        let args: Vec<_> = cmd.as_std().get_args().collect();
        assert_eq!(args.len(), 5);
    }

    #[test]
    fn build_command_with_special_chars_in_args() {
        let spec = exec_spec(
            "echo",
            &["arg with spaces", "arg'with'quotes", "arg\"with\"double"],
        );
        let cmd = build_command(&spec, std::path::Path::new("."), &test_vars()).unwrap();
        let args: Vec<_> = cmd.as_std().get_args().collect();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "arg with spaces");
        assert_eq!(args[1], "arg'with'quotes");
        assert_eq!(args[2], "arg\"with\"double");
    }
}
