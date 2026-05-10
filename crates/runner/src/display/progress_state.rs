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

use crate::command::OutputLine;
use indicatif::ProgressBar;
use ops_core::config::CommandId;
use std::collections::{HashMap, VecDeque};

/// Per-plan progress state: step bars, captured stderr, and id → display
/// mapping. Lifecycle is run-plan-scoped: filled by `on_plan_started`,
/// drained on `RunFinished`.
///
/// `step_stderr` is a bounded ring per id sized by the caller-supplied cap
/// in `record_stderr`. PERF-1 / TASK-0539: prior implementation held every
/// captured stderr line for the plan's lifetime even though only the
/// configured tail (`stderr_tail_lines`, default 5) is ever rendered.
pub(crate) struct ProgressState {
    pub bars: Vec<ProgressBar>,
    pub steps: Vec<(String, String)>,
    pub step_stderr: HashMap<String, VecDeque<OutputLine>>,
    pub display_map: HashMap<String, String>,
    pub plan_command_ids: Vec<String>,
    /// PERF-12 (TASK-0723): O(1) `id -> steps` index. Populated by
    /// [`Self::reset_for_plan`] alongside `steps`; queried by
    /// [`Self::step_index`] so the per-RunnerEvent lookup does not linearly
    /// scan a 32-step plan with thousands of stderr lines per step.
    ///
    /// PATTERN-1 (TASK-1109): the value is a queue of *remaining* step
    /// positions for that id, not a single position. A composite that fans
    /// the same leaf twice (TASK-0997: parallel orchestrator counts
    /// occurrences instead of dedup'ing by HashSet) would otherwise
    /// last-write-wins on duplicate ids, leaving the first bar permanently
    /// pending while the second received doubled `StepStarted`/`StepFinished`
    /// updates. [`Self::step_index`] now peeks the front of the queue (used
    /// by non-terminal events like `StepStarted`/`StepOutput`) and
    /// [`Self::consume_step_index`] pops it (called from the terminal
    /// `finish_step` path so the next occurrence routes to the next bar).
    pub index_by_id: HashMap<String, VecDeque<usize>>,
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
            index_by_id: HashMap::new(),
        }
    }

    /// Look up the bar/step row index for a given command id. Returns
    /// `None` if no step with that id is registered (e.g. an event arrived
    /// after the plan finished, or every occurrence of a duplicated id has
    /// already reached a terminal state and been consumed).
    ///
    /// O(1): hits `index_by_id` directly. The map is rebuilt on every
    /// `reset_for_plan`; tests that mutate `steps` outside that path must
    /// also update `index_by_id` for `step_index` to stay consistent.
    ///
    /// PATTERN-1 (TASK-1109): peeks at the front of the per-id queue. For
    /// duplicate ids the *current* (oldest still-running) occurrence is
    /// returned; a subsequent terminal event must call
    /// [`Self::consume_step_index`] to advance to the next occurrence.
    pub fn step_index(&self, id: &str) -> Option<usize> {
        self.index_by_id.get(id).and_then(|q| q.front().copied())
    }

    /// Pop the next step row index for a command id. Called from the
    /// terminal-event path (`finish_step` for `Succeeded` / `Failed` /
    /// `Skipped`) so duplicate ids advance to the next bar instead of
    /// last-write-wins'ing onto a single row.
    ///
    /// Returns the index that was active *before* the pop (i.e. the same
    /// value the prior `step_index` call would have returned), or `None`
    /// if the id has no remaining occurrences.
    pub fn consume_step_index(&mut self, id: &str) -> Option<usize> {
        let q = self.index_by_id.get_mut(id)?;
        let idx = q.pop_front()?;
        if q.is_empty() {
            // Keep the map tidy so `step_index` returns None promptly for
            // ids whose every occurrence has finished.
            self.index_by_id.remove(id);
        }
        Some(idx)
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

    /// Append a stderr line for the given step, bounded by `cap` so peak
    /// memory is O(tail) rather than O(captured stderr). Used by
    /// `on_step_output` to accumulate the tail that error-detail rendering
    /// consumes on failure. `cap == 0` records nothing.
    pub fn record_stderr(&mut self, id: &str, line: OutputLine, cap: usize) {
        if cap == 0 {
            return;
        }
        // PATTERN-1 (TASK-1178 / TASK-0998): single Entry lookup so each call
        // probes the map exactly once. The previous shape did two probes on
        // the cold-id path (`get_mut` miss, then `entry(...).or_default()`),
        // matching the dual-lookup bug TASK-0998 already cleaned up in
        // `merge_alias_for`. The trade-off: on the hit path we now always
        // allocate `id.to_string()` for the key. That allocation is bounded
        // by the per-line stderr work (one short id per output line) and the
        // pattern parity with the rest of the crate is the higher-value
        // invariant.
        let buf = self.step_stderr.entry(id.to_string()).or_default();
        if buf.len() == cap {
            buf.pop_front();
        }
        buf.push_back(line);
    }

    /// Replace the per-plan `steps` list and the parallel `plan_command_ids`
    /// from a fresh `command_ids` array. Clears any previous run's state.
    pub fn reset_for_plan(&mut self, command_ids: &[CommandId]) {
        self.steps = command_ids
            .iter()
            .map(|id| self.resolve_step_display(id))
            .collect();
        self.plan_command_ids = command_ids.iter().map(|id| id.to_string()).collect();
        self.index_by_id.clear();
        self.index_by_id.reserve(self.steps.len());
        for (idx, (sid, _)) in self.steps.iter().enumerate() {
            // PATTERN-1 (TASK-1109): append to a per-id queue rather than
            // overwriting. A naive `insert(sid, idx)` would silently
            // last-write-wins when a plan repeats an id (legal in parallel
            // composites — see TASK-0997), routing every event for that id
            // to the *last* row and leaving the first bar permanently
            // pending. We log a warn breadcrumb on each duplicate so
            // operators can correlate display oddities with plan shape.
            let entry = self.index_by_id.entry(sid.clone()).or_default();
            if !entry.is_empty() {
                tracing::warn!(
                    id = %sid,
                    occurrence = entry.len() + 1,
                    "ProgressState::reset_for_plan: duplicate command id; allocating an additional bar (TASK-1109)"
                );
            }
            entry.push_back(idx);
        }
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
        s.reset_for_plan(&["a".into(), "b".into()]);
        assert_eq!(s.step_index("a"), Some(0));
        assert_eq!(s.step_index("b"), Some(1));
        assert_eq!(s.step_index("missing"), None);
    }

    /// PERF-12 (TASK-0723): step_index hits the O(1) HashMap index instead
    /// of linearly scanning `steps`. We can't observe the scan directly, so
    /// we pin behavioural equivalence under a 32-step plan with many
    /// repeated lookups: every id resolves correctly, and unknown ids
    /// continue to return None even though the index is populated.
    #[test]
    fn step_index_resolves_via_o1_map_for_large_plan() {
        let ids: Vec<CommandId> = (0..32)
            .map(|i| CommandId::from(format!("step{i}")))
            .collect();
        let mut s = ProgressState::new(HashMap::new());
        s.reset_for_plan(&ids);
        for (i, id) in ids.iter().enumerate() {
            for _ in 0..32 {
                assert_eq!(s.step_index(id.as_str()), Some(i));
            }
        }
        assert_eq!(s.step_index("missing"), None);
        // After reset for a smaller plan, the index does not retain entries
        // from the previous plan (would otherwise leak across runs).
        s.reset_for_plan(&[CommandId::from("solo")]);
        assert_eq!(s.step_index("step0"), None);
        assert_eq!(s.step_index("solo"), Some(0));
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
        s.record_stderr("a", "line1".into(), 16);
        s.record_stderr("a", "line2".into(), 16);
        s.record_stderr("b", "other".into(), 16);
        let a: Vec<String> = s.step_stderr["a"]
            .iter()
            .map(|l| l.as_str().to_string())
            .collect();
        assert_eq!(a, vec!["line1".to_string(), "line2".to_string()]);
        let b: Vec<String> = s.step_stderr["b"]
            .iter()
            .map(|l| l.as_str().to_string())
            .collect();
        assert_eq!(b, vec!["other".to_string()]);
    }

    #[test]
    fn record_stderr_bounded_ring_keeps_only_tail() {
        let mut s = ProgressState::new(HashMap::new());
        // PERF-1 regression: stream 100k lines through a small ring and
        // confirm peak memory stays at the cap and only the tail is kept.
        let cap = 5;
        for i in 0..100_000 {
            s.record_stderr("noisy", format!("line {i}").into(), cap);
            assert!(s.step_stderr["noisy"].len() <= cap);
        }
        let tail: Vec<String> = s.step_stderr["noisy"]
            .iter()
            .map(|l| l.as_str().to_string())
            .collect();
        assert_eq!(
            tail,
            vec![
                "line 99995".to_string(),
                "line 99996".to_string(),
                "line 99997".to_string(),
                "line 99998".to_string(),
                "line 99999".to_string(),
            ]
        );
    }

    #[test]
    fn record_stderr_high_cap_preserves_full_tail() {
        // Verbose mode (raised cap) still preserves the full captured stream.
        let mut s = ProgressState::new(HashMap::new());
        for i in 0..1_000 {
            s.record_stderr("verbose", format!("v{i}").into(), usize::MAX);
        }
        assert_eq!(s.step_stderr["verbose"].len(), 1_000);
    }

    #[test]
    fn record_stderr_zero_cap_records_nothing() {
        let mut s = ProgressState::new(HashMap::new());
        s.record_stderr("a", "ignored".into(), 0);
        assert!(!s.step_stderr.contains_key("a"));
    }

    #[test]
    fn reset_for_plan_clears_previous_state_and_seeds_steps() {
        let mut s = ProgressState::new(HashMap::new());
        s.steps = vec![("old".into(), "old".into())];
        s.step_stderr
            .insert("old".into(), VecDeque::from(vec!["leak".into()]));
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

    /// PATTERN-1 (TASK-1109): a plan with a duplicated command id (legal in
    /// parallel composites — see TASK-0997) must allocate a distinct bar
    /// per occurrence. The previous `HashMap<String, usize>` silently
    /// last-write-wins'd, so every event for "x" routed to the second bar
    /// and the first bar sat as "pending" forever. Now `step_index` peeks
    /// the front of a per-id queue and `consume_step_index` pops it,
    /// mirroring the orchestrator's occurrence-counting in TASK-0997.
    #[test]
    fn duplicate_ids_route_to_distinct_bars_via_consume() {
        let mut s = ProgressState::new(HashMap::new());
        s.reset_for_plan(&[
            CommandId::from("x"),
            CommandId::from("y"),
            CommandId::from("x"),
        ]);
        // Both "x" rows exist as separate slots in `steps`.
        assert_eq!(s.steps.len(), 3);
        assert_eq!(s.steps[0].0, "x");
        assert_eq!(s.steps[2].0, "x");
        // First occurrence: peek then consume (terminal event).
        assert_eq!(s.step_index("x"), Some(0));
        assert_eq!(s.consume_step_index("x"), Some(0));
        // Second occurrence is now active — would be index 2 with the bug
        // (first occurrence would have been overwritten and lost).
        assert_eq!(s.step_index("x"), Some(2));
        assert_eq!(s.consume_step_index("x"), Some(2));
        // Both occurrences consumed → id is no longer routable.
        assert_eq!(s.step_index("x"), None);
        assert_eq!(s.consume_step_index("x"), None);
        // Untouched id still routes correctly.
        assert_eq!(s.step_index("y"), Some(1));
    }

    /// PATTERN-1 (TASK-1109): consume on a never-registered id is a no-op
    /// (mirrors the `step_index` contract — events arriving after a plan
    /// finished must not panic).
    #[test]
    fn consume_step_index_on_unknown_id_returns_none() {
        let mut s = ProgressState::new(HashMap::new());
        s.reset_for_plan(&[CommandId::from("a")]);
        assert_eq!(s.consume_step_index("missing"), None);
        // The known id is unaffected.
        assert_eq!(s.step_index("a"), Some(0));
    }
}
