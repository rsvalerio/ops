//! Per-plan progress bookkeeping extracted from [`super::ProgressDisplay`]
//! (ARCH-1 / TASK-0332).
//!
//! [`ProgressState`] owns the data the event-routing layer mutates as a
//! plan executes:
//!
//! - `bars` — one `indicatif::ProgressBar` per step row in display order
//! - `steps` — `(id, display_label)` pairs in plan order
//! - `step_stderr` — captured stderr lines per step id, used by error-detail rendering
//! - `display_map` — caller-supplied id → display-string overrides (read-only after construction)
//! - `plan_command_ids` — flat list of plan command ids, used by header/footer rendering
//!
//! The split keeps these tightly-coupled fields together (they are
//! always indexed in lock-step) and shrinks the surface of
//! `ProgressDisplay`, which still owns rendering config, IO/tap, and the
//! footer/header bars whose lifecycle is run-plan-scoped, not step-scoped.

use indicatif::ProgressBar;
use ops_core::config::CommandId;
use std::collections::HashMap;

/// Per-plan progress state: step bars, captured stderr, and id → display
/// mapping. Lifecycle is run-plan-scoped: filled by `on_plan_started`,
/// drained on `RunFinished`.
pub(crate) struct ProgressState {
    pub bars: Vec<ProgressBar>,
    pub steps: Vec<(String, String)>,
    pub step_stderr: HashMap<String, Vec<String>>,
    pub display_map: HashMap<String, String>,
    pub plan_command_ids: Vec<String>,
}

impl ProgressState {
    /// Construct a state seeded with caller-supplied id → display overrides.
    /// All other collections start empty; they are populated when a plan starts.
    pub fn new(display_map: HashMap<String, String>) -> Self {
        Self {
            bars: Vec::new(),
            steps: Vec::new(),
            step_stderr: HashMap::new(),
            display_map,
            plan_command_ids: Vec::new(),
        }
    }

    /// Look up the bar/step row index for a given command id. Returns
    /// `None` if no step with that id is registered (e.g. an event arrived
    /// after the plan finished).
    pub fn step_index(&self, id: &str) -> Option<usize> {
        self.steps.iter().position(|(sid, _)| sid == id)
    }

    /// Resolve a `CommandId` to its `(id_string, display_string)` pair,
    /// applying the configured `display_map` override and falling back to
    /// the id itself.
    pub fn resolve_step_display(&self, id: &CommandId) -> (String, String) {
        let id_str = id.to_string();
        let display = self
            .display_map
            .get(id.as_str())
            .cloned()
            .unwrap_or_else(|| {
                tracing::trace!(id = %id, "display_map fallback: using id as display");
                id_str.clone()
            });
        (id_str, display)
    }

    /// Append a stderr line for the given step. Used by `on_step_output`
    /// to accumulate the tail that error-detail rendering consumes on
    /// failure.
    pub fn record_stderr(&mut self, id: &str, line: String) {
        self.step_stderr
            .entry(id.to_string())
            .or_default()
            .push(line);
    }

    /// Replace the per-plan `steps` list and the parallel `plan_command_ids`
    /// from a fresh `command_ids` array. Clears any previous run's state.
    pub fn reset_for_plan(&mut self, command_ids: &[CommandId]) {
        self.steps = command_ids
            .iter()
            .map(|id| self.resolve_step_display(id))
            .collect();
        self.plan_command_ids = command_ids.iter().map(|id| id.to_string()).collect();
        self.bars.clear();
        self.step_stderr.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_index_returns_position_of_registered_id() {
        let mut s = ProgressState::new(HashMap::new());
        s.steps = vec![("a".into(), "A".into()), ("b".into(), "B".into())];
        assert_eq!(s.step_index("a"), Some(0));
        assert_eq!(s.step_index("b"), Some(1));
        assert_eq!(s.step_index("missing"), None);
    }

    #[test]
    fn resolve_step_display_uses_override_when_present() {
        let mut map = HashMap::new();
        map.insert("build".to_string(), "cargo build".to_string());
        let s = ProgressState::new(map);
        let (id, display) = s.resolve_step_display(&"build".into());
        assert_eq!(id, "build");
        assert_eq!(display, "cargo build");
    }

    #[test]
    fn resolve_step_display_falls_back_to_id_when_no_override() {
        let s = ProgressState::new(HashMap::new());
        let (id, display) = s.resolve_step_display(&"missing".into());
        assert_eq!(id, "missing");
        assert_eq!(display, "missing");
    }

    #[test]
    fn record_stderr_accumulates_per_id() {
        let mut s = ProgressState::new(HashMap::new());
        s.record_stderr("a", "line1".into());
        s.record_stderr("a", "line2".into());
        s.record_stderr("b", "other".into());
        assert_eq!(s.step_stderr["a"], vec!["line1", "line2"]);
        assert_eq!(s.step_stderr["b"], vec!["other"]);
    }

    #[test]
    fn reset_for_plan_clears_previous_state_and_seeds_steps() {
        let mut s = ProgressState::new(HashMap::new());
        s.steps = vec![("old".into(), "old".into())];
        s.step_stderr.insert("old".into(), vec!["leak".into()]);
        s.plan_command_ids = vec!["old".into()];
        s.reset_for_plan(&["new".into()]);
        assert_eq!(s.steps, vec![("new".to_string(), "new".to_string())]);
        assert_eq!(s.plan_command_ids, vec!["new".to_string()]);
        assert!(
            s.step_stderr.is_empty(),
            "stderr from prior plan must not leak"
        );
        assert!(s.bars.is_empty());
    }
}
