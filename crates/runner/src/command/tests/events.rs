//! Tests for emit_output_events.

use super::*;

#[test]
fn emit_output_events_emits_stdout_and_stderr() {
    let mut events: Vec<RunnerEvent> = Vec::new();
    emit_output_events("test", "line1\nline2\n", "err1\n", &mut |e| events.push(e));

    let stdout_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: false, .. }))
        .collect();
    let stderr_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: true, .. }))
        .collect();

    assert_eq!(stdout_events.len(), 2);
    assert_eq!(stderr_events.len(), 1);
}

/// TQ-018: Tests for emit_output_events edge cases.
mod emit_output_edge_tests {
    use super::*;

    #[test]
    fn emit_output_events_with_very_long_line() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let long_line = "x".repeat(100_000);
        emit_output_events("test", &long_line, "", &mut |e| events.push(e));

        assert_eq!(events.len(), 1);
        if let RunnerEvent::StepOutput { line, .. } = &events[0] {
            assert_eq!(line.len(), 100_000);
        } else {
            panic!("expected StepOutput event");
        }
    }

    #[test]
    fn emit_output_events_with_many_lines() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let many_lines: String = (0..1000).map(|i| format!("line{}\n", i)).collect();
        emit_output_events("test", &many_lines, "", &mut |e| events.push(e));

        assert_eq!(events.len(), 1000);
    }

    #[test]
    fn emit_output_events_with_unicode() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let unicode = "\u{65E5}\u{672C}\u{8A9E}\n\u{30C6}\u{30B9}\u{30C8}\n\u{1F389}\n";
        emit_output_events("test", unicode, "", &mut |e| events.push(e));

        assert_eq!(events.len(), 3);
    }
}
