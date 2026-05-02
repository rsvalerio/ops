//! Step-line shared types and duration formatting.

/// Format a duration in seconds into a human-friendly string.
///
/// - `< 60s` → `"0.74s"`, `"5.37s"` (two decimal places)
/// - `≥ 60s` → `"2m14s"`, `"4m38s"` (minutes + whole seconds)
/// - `≥ 3600s` → `"1h2m3s"` (hours + minutes + seconds)
///
/// SEC-15 / TASK-0358: NaN, negative, and infinite inputs render as `"--"`
/// rather than silently saturating through `as u64` casts.
pub fn format_duration(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--".to_string();
    }
    if secs < 60.0 {
        return format!("{:.2}s", secs);
    }
    // Truncate to whole seconds (matching the historical `as u64` floor) but
    // route through i128 so an enormous f64 saturates to u64::MAX instead of
    // silently wrapping or panicking.
    let total_secs = u64::try_from(secs.trunc() as i128).unwrap_or(u64::MAX);
    if total_secs < 3600 {
        let mins = total_secs / 60;
        let remaining = total_secs % 60;
        format!("{}m{}s", mins, remaining)
    } else {
        let hours = total_secs / 3600;
        let remaining = total_secs % 3600;
        let mins = remaining / 60;
        let secs_part = remaining % 60;
        format!("{}h{}m{}s", hours, mins, secs_part)
    }
}

/// Snapshot of run-plan progress passed to the boxed layout border methods.
///
/// Grouping these fields into a struct keeps method signatures narrow
/// (clippy `too_many_arguments`) and lets the caller compute each value once.
#[derive(Debug, Clone, Copy)]
pub struct BoxSnapshot<'a> {
    /// Number of steps in a terminal state so far (CL-3 / TASK-0771: this
    /// includes failed and skipped, not only successful — the "completed"
    /// label is retained for backwards compatibility).
    pub completed: usize,
    /// Steps that ended in `StepStatus::Failed`. Used by the bottom border
    /// to surface "F failed of T" rather than the legacy "Done N/M" line.
    pub failed: usize,
    /// Steps that ended in `StepStatus::Skipped` (cancelled, fail_fast
    /// orphans, …). Distinguished from failed so summary lines can read
    /// "S succeeded, K skipped, F failed of T".
    pub skipped: usize,
    /// Total steps in the plan.
    pub total: usize,
    /// Elapsed seconds since the plan started (wall-clock).
    pub elapsed_secs: f64,
    /// Whether the run has been fully successful up to this point.
    pub success: bool,
    /// Terminal columns available for the border.
    pub columns: u16,
    /// Command IDs of the plan, for headers that list them (e.g. `Running: build, test`).
    pub command_ids: &'a [String],
}

// `BoxSnapshot` is a plain value-type bag with one field per piece of plan
// state, intentionally constructed via struct-literal syntax at call sites
// so each field is named at the use site and clippy's too_many_arguments
// rule (threshold 5) is respected without an `#[allow]`.

/// Plain layout pieces that make up the left portion of a step line:
/// `{indent}{icon}{pad} `. Returned by `ConfigurableTheme::step_prefix_parts`
/// so `render` and `render_prefix` cannot drift in width or composition.
pub struct StepPrefixParts<'a> {
    /// Leading indent (empty for running rows; spinner template emits its own indent).
    pub indent: &'a str,
    /// Status icon glyph.
    pub icon: &'a str,
    /// Spaces padding the icon column to `icon_column_width`.
    pub pad: String,
}
