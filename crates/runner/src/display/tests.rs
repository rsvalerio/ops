use super::*;
use crate::command::RunnerEvent;
use indexmap::IndexMap;
use ops_core::config;
use ops_core::output::{StepLine, StepStatus};
use ops_theme::ThemeConfig;
use std::collections::HashMap;

/// Renders step lines with status icons and elapsed time.
pub struct StepRenderer<'a> {
    theme: &'a theme::ConfigurableTheme,
    columns: u16,
}

impl<'a> StepRenderer<'a> {
    pub fn new(theme: &'a theme::ConfigurableTheme, columns: u16) -> Self {
        Self { theme, columns }
    }

    pub fn render(&self, status: StepStatus, label: &str, elapsed: Option<f64>) -> String {
        let step = StepLine::new(status, label.to_string(), elapsed);
        self.theme.render(&step, self.columns)
    }
}

fn test_themes() -> IndexMap<String, ThemeConfig> {
    let mut themes = IndexMap::new();
    themes.insert("classic".into(), ThemeConfig::classic());
    themes.insert("compact".into(), ThemeConfig::compact());
    themes
}

/// DUP-004: Reduce repeated ProgressDisplay test setup.
fn test_display(entries: &[(&str, &str)]) -> ProgressDisplay {
    test_display_with_config(config::OutputConfig::default(), entries)
}

fn test_display_with_config(
    output: config::OutputConfig,
    entries: &[(&str, &str)],
) -> ProgressDisplay {
    let display_map: HashMap<String, String> = entries
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let custom_themes = test_themes();
    ProgressDisplay::new(DisplayOptions {
        output: &output,
        display_map,
        custom_themes: &custom_themes,
        tap: None,
        verbose: false,
    })
    .expect("test display construct")
}

#[test]
fn progress_display_handles_full_lifecycle() {
    let mut display = test_display(&[("echo_hi", "echo hi")]);

    // Non-TTY: events go to stderr (we just verify no panics and state is correct)
    display.handle_event(RunnerEvent::PlanStarted {
        command_ids: vec!["echo_hi".into()],
    });
    assert_eq!(display.state.steps.len(), 1);
    assert_eq!(display.step_index("echo_hi"), Some(0));
    assert_eq!(display.step_index("unknown"), None);

    // TQ-004: Verify the rendered pending step line contains the label text
    let pending_msg = display.state.bars[0].message();
    assert!(
        pending_msg.contains("echo hi"),
        "pending step line should contain label, got: {pending_msg}"
    );

    display.handle_event(RunnerEvent::StepStarted {
        id: "echo_hi".into(),
        display_cmd: Some("echo hi".to_string()),
    });

    display.handle_event(RunnerEvent::StepOutput {
        id: "echo_hi".into(),
        line: "some error output".into(),
        stderr: true,
    });
    assert_eq!(display.state.step_stderr["echo_hi"].len(), 1);

    display.handle_event(RunnerEvent::StepFinished {
        id: "echo_hi".into(),
        duration_secs: 0.05,
        display_cmd: Some("echo hi".to_string()),
    });

    display.handle_event(RunnerEvent::RunFinished {
        duration_secs: 0.05,
        success: true,
    });
}

#[test]
fn progress_display_handles_failure_with_error_detail() {
    let mut display = test_display_with_config(
        config::OutputConfig {
            show_error_detail: true,
            ..config::OutputConfig::default()
        },
        &[("fail_cmd", "false")],
    );

    display.handle_event(RunnerEvent::PlanStarted {
        command_ids: vec!["fail_cmd".into()],
    });
    display.handle_event(RunnerEvent::StepStarted {
        id: "fail_cmd".into(),
        display_cmd: Some("false".to_string()),
    });
    display.handle_event(RunnerEvent::StepOutput {
        id: "fail_cmd".into(),
        line: "error: something went wrong".into(),
        stderr: true,
    });
    display.handle_event(RunnerEvent::StepFailed {
        id: "fail_cmd".into(),
        duration_secs: 0.01,
        message: "exit status: 1".to_string(),
        display_cmd: Some("false".to_string()),
    });
    display.handle_event(RunnerEvent::RunFinished {
        duration_secs: 0.01,
        success: false,
    });
    // Verify stderr was captured
    assert_eq!(display.state.step_stderr["fail_cmd"].len(), 1);
}

#[test]
fn progress_display_render_step() {
    let display = test_display(&[]);
    let renderer = StepRenderer::new(
        &display.render_config().theme,
        display.render_config().columns,
    );
    let line = renderer.render(StepStatus::Succeeded, "cargo build", Some(1.23));
    assert!(line.contains("cargo build"));
    assert!(line.contains("1.23s"));
}

#[test]
fn emit_line_non_tty_writes_to_stderr() {
    let output = config::OutputConfig {
        columns: 80,
        ..config::OutputConfig::default()
    };
    let custom_themes = test_themes();
    let display = ProgressDisplay::new_with_tty_check(
        DisplayOptions {
            output: &output,
            display_map: HashMap::new(),
            custom_themes: &custom_themes,
            tap: None,
            verbose: false,
        },
        || false,
    )
    .expect("should construct");
    assert!(!display.render.is_tty);
    display.emit_line("test line");
}

#[test]
fn tap_file_captures_raw_output() {
    // TEST-20: use a per-test tempdir instead of a shared
    // `std::env::temp_dir().join("ops_tap_test.log")`, which two
    // concurrent `cargo test` invocations would race on.
    let dir = tempfile::tempdir().expect("tempdir");
    let tap_path = dir.path().join("ops_tap_test.log");
    let output = config::OutputConfig::default();
    let display_map: HashMap<String, String> = [("cmd", "echo hello")]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let custom_themes = test_themes();
    let mut display = ProgressDisplay::new(DisplayOptions {
        output: &output,
        display_map,
        custom_themes: &custom_themes,
        tap: Some(tap_path.clone()),
        verbose: false,
    })
    .expect("should construct with tap");

    display.handle_event(RunnerEvent::PlanStarted {
        command_ids: vec!["cmd".into()],
    });
    display.handle_event(RunnerEvent::StepOutput {
        id: "cmd".into(),
        line: "hello world".into(),
        stderr: false,
    });
    display.handle_event(RunnerEvent::StepOutput {
        id: "cmd".into(),
        line: "error line".into(),
        stderr: true,
    });

    // Drop to flush
    drop(display);

    let contents = std::fs::read_to_string(&tap_path).expect("read tap file");
    assert!(
        contents.contains("hello world"),
        "tap should contain stdout: {contents}"
    );
    assert!(
        contents.contains("error line"),
        "tap should contain stderr: {contents}"
    );

    // tempdir is cleaned up on drop; no manual remove_file needed.
}

#[test]
fn tap_none_produces_no_file() {
    let display = test_display(&[("cmd", "test")]);
    assert!(display.tap.is_none());
}

#[test]
fn step_stderr_captures_output() {
    let mut display = test_display(&[("cmd", "test cmd")]);

    display.handle_event(RunnerEvent::PlanStarted {
        command_ids: vec!["cmd".into()],
    });
    display.handle_event(RunnerEvent::StepOutput {
        id: "cmd".into(),
        line: "stderr line 1".into(),
        stderr: true,
    });
    display.handle_event(RunnerEvent::StepOutput {
        id: "cmd".into(),
        line: "stdout line".into(),
        stderr: false,
    });
    display.handle_event(RunnerEvent::StepOutput {
        id: "cmd".into(),
        line: "stderr line 2".into(),
        stderr: true,
    });

    let captured = display
        .state
        .step_stderr
        .get("cmd")
        .expect("should capture stderr");
    assert_eq!(captured.len(), 2);
    assert_eq!(captured[0].as_str(), "stderr line 1");
    assert_eq!(captured[1].as_str(), "stderr line 2");
}

#[test]
fn render_config_uses_output_settings() {
    let display = test_display_with_config(
        config::OutputConfig {
            columns: 100,
            show_error_detail: false,
            theme: "compact".into(),
            stderr_tail_lines: 10,
            category_order: Vec::new(),
        },
        &[],
    );
    assert_eq!(display.render.columns, 100);
    assert!(!display.render.show_error_detail);
    assert_eq!(display.render.stderr_tail, StderrTail::Limited(10));
}

/// TASK-0762: user's `stderr_tail_lines` config value is preserved when verbose
/// is true — verbose overrides at the display layer via `StderrTail::Unbounded`
/// without mutating the underlying config.
#[test]
fn verbose_overrides_to_unbounded_without_mutating_config() {
    let output = config::OutputConfig {
        columns: 80,
        show_error_detail: true,
        theme: "compact".into(),
        stderr_tail_lines: 1000,
        category_order: Vec::new(),
    };
    let display_map = HashMap::new();
    let custom_themes = test_themes();
    let display = ProgressDisplay::new(DisplayOptions {
        output: &output,
        display_map,
        custom_themes: &custom_themes,
        tap: None,
        verbose: true,
    })
    .expect("test display construct");
    // Verbose → Unbounded, not Limited(usize::MAX)
    assert_eq!(display.render.stderr_tail, StderrTail::Unbounded);
    // The original config is unchanged (not mutated)
    assert_eq!(output.stderr_tail_lines, 1000);
}

#[test]
fn progress_display_handles_step_skipped() {
    let mut display = test_display(&[("skip_cmd", "skipped command")]);

    display.handle_event(RunnerEvent::PlanStarted {
        command_ids: vec!["skip_cmd".into()],
    });
    display.handle_event(RunnerEvent::StepStarted {
        id: "skip_cmd".into(),
        display_cmd: Some("skipped command".to_string()),
    });
    display.handle_event(RunnerEvent::StepSkipped {
        id: "skip_cmd".into(),
        display_cmd: Some("skipped command".to_string()),
    });
    display.handle_event(RunnerEvent::RunFinished {
        duration_secs: 0.0,
        success: true,
    });

    assert!(display.state.bars.len() == 1);
}

mod edge_case_tests {
    use super::*;
    use crate::command::RunnerEvent;
    use ops_core::output::StepStatus;

    #[test]
    fn extract_stderr_tail_extracts_correct_count() {
        let lines: Vec<crate::command::OutputLine> =
            (1..=10).map(|i| format!("line {}", i).into()).collect();
        let tail = ErrorDetailRenderer::extract_stderr_tail(&lines, DEFAULT_STDERR_TAIL_LINES);
        assert_eq!(tail.len(), DEFAULT_STDERR_TAIL_LINES);
        assert_eq!(tail[0], "line 6");
        assert_eq!(tail[4], "line 10");
    }

    #[test]
    fn extract_stderr_tail_handles_fewer_lines() {
        let lines: Vec<crate::command::OutputLine> = vec!["a".into(), "b".into()];
        let tail = ErrorDetailRenderer::extract_stderr_tail(&lines, DEFAULT_STDERR_TAIL_LINES);
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0], "a");
        assert_eq!(tail[1], "b");
    }

    #[test]
    fn extract_stderr_tail_handles_empty() {
        let lines: Vec<crate::command::OutputLine> = vec![];
        let tail = ErrorDetailRenderer::extract_stderr_tail(&lines, DEFAULT_STDERR_TAIL_LINES);
        assert!(tail.is_empty());
    }

    #[test]
    fn extract_stderr_tail_unlimited_returns_all() {
        let lines: Vec<crate::command::OutputLine> =
            (1..=100).map(|i| format!("line {}", i).into()).collect();
        let tail = ErrorDetailRenderer::extract_stderr_tail(&lines, usize::MAX);
        assert_eq!(tail.len(), 100);
    }

    #[test]
    fn finish_step_returns_none_for_unknown_id() {
        let mut display = test_display(&[]);

        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["known".into()],
        });

        let result = display.finish_step("unknown", StepStatus::Succeeded, 1.0, None);
        assert!(result.is_none(), "unknown step should return None");
    }

    #[test]
    fn write_stderr_handles_none_and_some() {
        // Verifies no-panic on both None and Some inputs.
        write_stderr(None);
        write_stderr(Some("test line"));
    }
}

/// TQ-018: Test rapid concurrent event sequences don't cause panics or race conditions.
mod concurrent_event_tests {
    use super::*;

    #[test]
    fn handle_event_rapid_sequence_no_panic() {
        let mut display = test_display(&[("cmd1", "echo 1"), ("cmd2", "echo 2")]);

        // Simulate rapid event sequence as would occur in parallel execution
        let events = vec![
            RunnerEvent::PlanStarted {
                command_ids: vec!["cmd1".into(), "cmd2".into()],
            },
            RunnerEvent::StepStarted {
                id: "cmd1".into(),
                display_cmd: Some("echo 1".into()),
            },
            RunnerEvent::StepStarted {
                id: "cmd2".into(),
                display_cmd: Some("echo 2".into()),
            },
            RunnerEvent::StepOutput {
                id: "cmd1".into(),
                line: "output1".into(),
                stderr: false,
            },
            RunnerEvent::StepOutput {
                id: "cmd2".into(),
                line: "output2".into(),
                stderr: true,
            },
            RunnerEvent::StepFinished {
                id: "cmd1".into(),
                duration_secs: 0.1,
                display_cmd: Some("echo 1".into()),
            },
            RunnerEvent::StepFinished {
                id: "cmd2".into(),
                duration_secs: 0.15,
                display_cmd: Some("echo 2".into()),
            },
            RunnerEvent::RunFinished {
                duration_secs: 0.2,
                success: true,
            },
        ];

        // Handle all events - should not panic
        for event in events {
            display.handle_event(event);
        }

        // Verify state is consistent
        assert_eq!(display.state.steps.len(), 2);
        assert_eq!(display.state.bars.len(), 2);
    }

    #[test]
    fn handle_event_interleaved_failure_sequence() {
        let mut display = test_display_with_config(
            config::OutputConfig {
                show_error_detail: true,
                ..config::OutputConfig::default()
            },
            &[("ok", "true"), ("fail", "false")],
        );

        // Simulate parallel execution with one failure
        let events = vec![
            RunnerEvent::PlanStarted {
                command_ids: vec!["ok".into(), "fail".into()],
            },
            RunnerEvent::StepStarted {
                id: "ok".into(),
                display_cmd: Some("true".into()),
            },
            RunnerEvent::StepStarted {
                id: "fail".into(),
                display_cmd: Some("false".into()),
            },
            RunnerEvent::StepOutput {
                id: "fail".into(),
                line: "error message".into(),
                stderr: true,
            },
            RunnerEvent::StepFinished {
                id: "ok".into(),
                duration_secs: 0.01,
                display_cmd: Some("true".into()),
            },
            RunnerEvent::StepFailed {
                id: "fail".into(),
                duration_secs: 0.01,
                message: "exit status 1".into(),
                display_cmd: Some("false".into()),
            },
            RunnerEvent::RunFinished {
                duration_secs: 0.02,
                success: false,
            },
        ];

        for event in events {
            display.handle_event(event);
        }

        // Verify stderr was captured for failed command
        assert!(display.state.step_stderr.contains_key("fail"));
    }
}

/// TQ-005: Test ProgressDisplay error handling for invalid theme/template.
mod error_path_tests {
    use super::*;

    #[test]
    fn progress_display_invalid_theme_returns_error() {
        let output = config::OutputConfig {
            theme: "nonexistent_theme".into(),
            ..config::OutputConfig::default()
        };
        let custom_themes = IndexMap::new();
        let result = ProgressDisplay::new(DisplayOptions {
            output: &output,
            display_map: HashMap::new(),
            custom_themes: &custom_themes,
            tap: None,
            verbose: false,
        });

        match result {
            Err(e) => {
                let err = e.to_string();
                assert!(
                    err.contains("Theme not found") || err.contains("nonexistent_theme"),
                    "error should mention theme issue: {err}"
                );
            }
            Ok(_) => panic!("should fail for nonexistent theme"),
        }
    }

    #[test]
    fn progress_display_valid_theme_succeeds() {
        let output = config::OutputConfig {
            theme: "classic".into(),
            ..config::OutputConfig::default()
        };
        // test_display_with_config uses test_themes() which includes "classic"
        let _display = test_display_with_config(output, &[]);
    }
}

/// TQ-013: Test handle_event with unknown command IDs.
mod unknown_command_tests {
    use super::*;

    #[test]
    fn handle_event_unknown_command_id_no_panic() {
        let mut display = test_display(&[]);

        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["known_cmd".into()],
        });

        display.handle_event(RunnerEvent::StepStarted {
            id: "unknown_cmd".into(),
            display_cmd: Some("unknown command".to_string()),
        });

        display.handle_event(RunnerEvent::StepFinished {
            id: "unknown_cmd".into(),
            duration_secs: 0.1,
            display_cmd: Some("unknown command".to_string()),
        });

        display.handle_event(RunnerEvent::RunFinished {
            duration_secs: 0.1,
            success: true,
        });
    }

    #[test]
    fn handle_event_step_output_for_unknown_command_no_panic() {
        let mut display = test_display(&[]);

        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["cmd1".into()],
        });

        display.handle_event(RunnerEvent::StepOutput {
            id: "non_existent_cmd".into(),
            line: "some output".into(),
            stderr: true,
        });

        assert!(
            display.state.step_stderr.contains_key("non_existent_cmd"),
            "output for unknown command should be stored under its ID"
        );
    }

    /// A running step that never receives a terminal event (e.g. its task was
    /// aborted under `fail_fast`) must still be finalized on `RunFinished` so
    /// its bar stays visible in the boxed frame instead of being dropped from
    /// the multi-progress draw.
    ///
    /// TASK-0329: this test exercises the orphan-finalization path beyond mere
    /// `is_finished()` liveness — it also asserts the rendered message reflects
    /// `StepStatus::Skipped` (so a regression that finalizes with Failed/Succeeded
    /// would be caught) and that the elapsed value flows from the bar's own
    /// timer rather than a hard-coded zero.
    ///
    /// TASK-0333: the footer count must include orphan-finalized rows so that
    /// `Done N/M` agrees with the number of rendered rows.
    #[test]
    fn run_finished_finalizes_orphan_running_bars() {
        let mut display = test_display(&[("a", "echo a"), ("b", "echo b")]);

        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["a".into(), "b".into()],
        });
        display.handle_event(RunnerEvent::StepStarted {
            id: "a".into(),
            display_cmd: Some("echo a".to_string()),
        });
        display.handle_event(RunnerEvent::StepStarted {
            id: "b".into(),
            display_cmd: Some("echo b".to_string()),
        });
        // Sleep briefly so the orphan bar accrues measurable elapsed time;
        // finalize_orphan_bars reads the bar's own timer (TASK-0337 / 0329 #2).
        std::thread::sleep(std::time::Duration::from_millis(15));
        display.handle_event(RunnerEvent::StepFinished {
            id: "a".into(),
            duration_secs: 0.1,
            display_cmd: Some("echo a".to_string()),
        });
        // Note: no terminal event for "b" — simulates an aborted task.
        display.handle_event(RunnerEvent::RunFinished {
            duration_secs: 0.1,
            success: true,
        });

        assert!(
            display.state.bars[0].is_finished(),
            "step a should be finished by its StepFinished event"
        );
        assert!(
            display.state.bars[1].is_finished(),
            "orphan running step b should be finalized on RunFinished"
        );

        // TASK-0329 #1 / #3: orphan bar's rendered message must carry the
        // Skipped icon (classic theme uses U+2298 "⊘"). This rules out a
        // regression where the bar is finalized with Failed (✗) or Succeeded.
        let orphan_msg = display.state.bars[1].message();
        assert!(
            orphan_msg.contains('\u{2298}'),
            "orphan bar message should contain the Skipped icon, got: {orphan_msg}"
        );
        assert!(
            !orphan_msg.contains('\u{2717}'),
            "orphan bar must not be finalized as Failed, got: {orphan_msg}"
        );

        // TASK-0329 #2: the rendered elapsed must be derived from the bar's
        // timer (so a non-trivial >0ms run shows a non-zero elapsed). The
        // classic theme renders elapsed as "Ns" / "Nms"; assert the string
        // does not collapse to a 0.0/0ms placeholder.
        assert!(
            !orphan_msg.contains(" 0.00s") && !orphan_msg.contains(" 0ms"),
            "orphan bar elapsed should be non-zero (read from bar timer), got: {orphan_msg}"
        );

        // TASK-0333: completed_steps must include the orphan-finalized row,
        // so Done 2/2 agrees with the two visible finished rows.
        assert_eq!(
            display.completed_steps, 2,
            "orphan-finalization should bump completed_steps so footer agrees with visible row count"
        );
    }

    /// TQ-012: finish_step with unknown step ID returns None.
    #[test]
    fn finish_step_unknown_id_returns_none() {
        let mut display = test_display(&[]);

        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["known".into()],
        });

        // finish_step is called internally via on_step_finished -- trigger it with unknown ID
        display.handle_event(RunnerEvent::StepFinished {
            id: "never_registered".into(),
            duration_secs: 1.0,
            display_cmd: None,
        });
        // No panic means finish_step correctly returned None and was handled
    }

    #[test]
    fn handle_event_step_failed_for_unknown_command_no_panic() {
        let mut display = test_display_with_config(
            config::OutputConfig {
                show_error_detail: true,
                ..config::OutputConfig::default()
            },
            &[],
        );

        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["known".into()],
        });

        display.handle_event(RunnerEvent::StepFailed {
            id: "unknown_failed".into(),
            duration_secs: 0.1,
            message: "exit status 1".to_string(),
            display_cmd: Some("unknown failed cmd".to_string()),
        });

        display.handle_event(RunnerEvent::RunFinished {
            duration_secs: 0.1,
            success: false,
        });
    }
}

// TRAIT-9 / TASK-1141: the `!Send` invariant assertion was moved out of
// this test-only module into `display.rs` itself so every compilation
// profile (not just `cargo test`) enforces it. See
// `assert_progress_display_not_send` there.
