//! Step-line shared types and duration formatting.

/// Format a duration in seconds into a human-friendly string.
///
/// - `< 60s` → `"0.74s"`, `"5.37s"` (two decimal places)
/// - `≥ 60s` → `"2m14s"`, `"4m38s"` (minutes + whole seconds)
/// - `≥ 3600s` → `"1h2m3s"` (hours + minutes + seconds)
///
/// NaN, negative, and infinite inputs render as `"--"` rather than
/// silently saturating through `as u64` casts.
#[must_use]
pub fn format_duration(secs: f64) -> String {
    if !secs.is_finite() || secs < 0.0 {
        return "--".to_string();
    }
    if secs < 60.0 {
        return format!("{secs:.2}s");
    }
    let total_secs = whole_secs(secs);
    if total_secs < 3600 {
        let mins = total_secs / 60;
        let remaining = total_secs % 60;
        format!("{mins}m{remaining}s")
    } else {
        let hours = total_secs / 3600;
        let remaining = total_secs % 3600;
        let mins = remaining / 60;
        let secs_part = remaining % 60;
        format!("{hours}h{mins}m{secs_part}s")
    }
}

/// Columns [`format_running_elapsed`] never exceeds: the fixed
/// `{elapsed:>6}` field every built-in `running_template` reserves.
pub const RUNNING_ELAPSED_WIDTH: usize = 6;

/// Format the live elapsed time of a running step in at most
/// [`RUNNING_ELAPSED_WIDTH`] columns.
///
/// Under an hour this is [`format_duration`] (`"0.74s"` .. `"59m59s"`, all
/// six columns or fewer). Past an hour `format_duration` grows to eight
/// columns (`"9h59m59s"`), which would push a running row past its column
/// budget, and a live counter needs no seconds at that scale, so:
///
/// - `< 100h` → `"1h00m"` .. `"99h59m"` (hours + minutes)
/// - `< 100d` → `"4d04h"` .. `"99d23h"` (days + hours)
/// - beyond → whole days, `"100d"` (six columns up to 99999 days)
///
/// Finished durations keep [`format_duration`]'s full precision; NaN,
/// negative and infinite inputs render as `"--"` as there.
#[must_use]
pub fn format_running_elapsed(secs: f64) -> String {
    if !secs.is_finite() || secs < 3600.0 {
        return format_duration(secs);
    }
    let total_secs = whole_secs(secs);
    let hours = total_secs / 3600;
    if hours < 100 {
        return format!("{hours}h{:02}m", (total_secs % 3600) / 60);
    }
    let days = total_secs / 86_400;
    if days < 100 {
        format!("{days}d{:02}h", (total_secs % 86_400) / 3600)
    } else {
        format!("{days}d")
    }
}

/// Whole seconds in `secs`, for a finite, non-negative `secs`.
///
/// Explicit clamp into the f64-representable u64 range before the lossy
/// `as u64` cast, so huge values saturate to `u64::MAX` visibly rather
/// than by accident. Callers reject NaN, negative and infinite input first.
const fn whole_secs(secs: f64) -> u64 {
    // The casts are the point: `u64::MAX as f64` rounds up to the nearest
    // representable f64 (the clamp bound), and `clamped as u64` saturates
    // there. Both directions are intended and bounded by the callers' guards.
    // `as_conversions` is allowed for the same reason: no `From`/`TryFrom` pair
    // expresses "round `u64::MAX` up to the nearest representable f64" or
    // "saturate an already-clamped f64 down to u64", so the `as` casts are the
    // only spelling of the intended, guard-bounded conversion.
    #[allow(
        clippy::as_conversions,
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let total_secs = {
        let clamped = secs.trunc().clamp(0.0, u64::MAX as f64);
        clamped as u64
    };
    total_secs
}

/// Snapshot of run-plan progress passed to the boxed layout border methods.
///
/// Grouping these fields into a struct keeps method signatures narrow
/// (clippy `too_many_arguments`) and lets the caller compute each value once.
#[derive(Debug, Clone, Copy)]
pub struct BoxSnapshot<'a> {
    /// Number of steps in a terminal state so far: failed and skipped
    /// included, not only successful. The `completed` field name is
    /// retained for backwards compatibility.
    pub completed: usize,
    /// Steps that ended in `StepStatus::Failed`. Used by the bottom border
    /// to surface "F failed of T" rather than the legacy "Done N/M" line.
    pub failed: usize,
    /// Steps that ended in `StepStatus::Skipped` (cancelled, `fail_fast`
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

/// Inputs to `ConfigurableTheme::render_slot` — the generalized
/// "left chrome + label + dotted separator + right-aligned trailing slot"
/// line shared by the runner's step line and report rows.
///
/// The runner builds one from a [`StepLine`](ops_core::output::StepLine) (icon
/// from the step status, trailing = formatted duration); a report builds one
/// per [`ReportRow`](ops_core::report::ReportRow) (icon/color from the theme's
/// `[report]` block, trailing = the result string). Keeping the right-hand slot
/// a plain string + precomputed SGR is what lets a single render path serve
/// both — see `ConfigurableTheme::render_slot`.
#[derive(Debug, Clone, Copy)]
pub struct SlotLine<'a> {
    /// Glyph for the icon column (theme step icon OR report status icon).
    pub icon: &'a str,
    /// Command label / section name shown after the icon.
    pub label: &'a str,
    /// Right-aligned trailing string: `"1.20s"` | `"None"` | `"28 warnings"` | `""`.
    pub trailing: &'a str,
    /// Precomputed SGR prefix for the trailing slot (`duration_color` for the
    /// runner; the per-row report color for reports). `None` renders plain.
    pub trailing_prefix: Option<&'a str>,
    /// Running rows drop the left pad and reserve spinner width.
    pub is_running: bool,
}

/// Plain layout pieces that make up the left portion of a step line:
/// `{indent}{icon}{pad} `.
///
/// Returned by `ConfigurableTheme::step_prefix_parts` so `render` and
/// `render_prefix` cannot drift in width or composition.
#[derive(Debug, Clone)]
pub struct StepPrefixParts<'a> {
    /// Leading indent (empty for running rows; spinner template emits its own indent).
    pub indent: &'a str,
    /// Status icon glyph.
    pub icon: &'a str,
    /// Spaces padding the icon column to `icon_column_width`.
    pub pad: String,
}
