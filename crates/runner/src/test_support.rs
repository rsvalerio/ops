//! Shared test utilities for runner-based tests.
//!
//! Gated behind the `test-support` feature so downstream crates can opt in.

use crate::command::CommandRunner;
use crate::command::RunnerEvent;
use ops_core::config::CommandSpec;
use std::collections::HashMap;
use std::path::PathBuf;

/// Create a CommandRunner with the given commands for testing.
pub fn test_runner(commands: HashMap<String, CommandSpec>) -> CommandRunner {
    let config = ops_core::test_utils::test_config_with_commands(commands);
    CommandRunner::new(config, PathBuf::from("."))
}

/// Helper trait for event assertions in tests.
pub trait EventAssertions {
    fn has_event<F>(&self, predicate: F) -> bool
    where
        F: Fn(&RunnerEvent) -> bool;

    fn assert_has_event<F>(&self, predicate: F, message: &str)
    where
        F: Fn(&RunnerEvent) -> bool;

    /// Check if a PlanStarted event exists.
    #[allow(dead_code)]
    fn has_plan_started(&self) -> bool {
        self.has_event(|e| matches!(e, RunnerEvent::PlanStarted { .. }))
    }

    /// Check if a StepFinished event exists for the given command ID.
    #[allow(dead_code)]
    fn has_step_finished(&self, id: &str) -> bool {
        self.has_event(
            |e| matches!(e, RunnerEvent::StepFinished { id: event_id, .. } if event_id == id),
        )
    }

    /// Check if a StepFailed event exists for the given command ID.
    #[allow(dead_code)]
    fn has_step_failed(&self, id: &str) -> bool {
        self.has_event(
            |e| matches!(e, RunnerEvent::StepFailed { id: event_id, .. } if event_id == id),
        )
    }

    /// Check if a StepSkipped event exists for the given command ID.
    #[allow(dead_code)]
    fn has_step_skipped(&self, id: &str) -> bool {
        self.has_event(
            |e| matches!(e, RunnerEvent::StepSkipped { id: event_id, .. } if event_id == id),
        )
    }

    /// Check if a RunFinished event exists with success=true.
    #[allow(dead_code)]
    fn has_run_finished_success(&self) -> bool {
        self.has_event(|e| matches!(e, RunnerEvent::RunFinished { success: true, .. }))
    }

    /// Check if a RunFinished event exists with success=false.
    #[allow(dead_code)]
    fn has_run_finished_failure(&self) -> bool {
        self.has_event(|e| matches!(e, RunnerEvent::RunFinished { success: false, .. }))
    }

    /// Count events matching a predicate.
    #[allow(dead_code)]
    fn count_events_matching<F>(&self, predicate: F) -> usize
    where
        F: Fn(&RunnerEvent) -> bool;
}

impl EventAssertions for Vec<RunnerEvent> {
    fn has_event<F>(&self, predicate: F) -> bool
    where
        F: Fn(&RunnerEvent) -> bool,
    {
        self.iter().any(predicate)
    }

    fn assert_has_event<F>(&self, predicate: F, message: &str)
    where
        F: Fn(&RunnerEvent) -> bool,
    {
        assert!(self.has_event(predicate), "{}", message);
    }

    fn count_events_matching<F>(&self, predicate: F) -> usize
    where
        F: Fn(&RunnerEvent) -> bool,
    {
        self.iter().filter(|e| predicate(e)).count()
    }
}
