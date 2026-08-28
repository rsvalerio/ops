//! Theme configuration types (serializable).
//!
//! These are the data types for theme configuration stored in TOML.
//! The rendering logic that uses these types lives in the theme crate.

use crate::output::StepStatus;
use crate::report::ReportStatus;
use serde::{Deserialize, Serialize};

/// Style for rendering the plan header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanHeaderStyle {
    /// Plain header: "Running: cmd1, cmd2"
    #[default]
    Plain,
    /// Tree-style header with box-drawing chars: "┌ Running: cmd1, cmd2" + "│"
    Tree,
}

/// Overall layout for step output.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LayoutKind {
    /// Classic flat layout: one line per step, footer summary.
    #[default]
    Flat,
    /// Full enclosing box with live header summary and a vertical progress column.
    Boxed,
}

/// Box-drawing characters for error detail blocks.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ErrorBlockChars {
    /// Top-left corner (e.g., "┌─" or "╭─")
    pub top: String,
    /// Vertical line for middle rows (e.g., "│")
    pub mid: String,
    /// Bottom-left corner (e.g., "└─" or "╰─")
    pub bottom: String,
    /// Rail character prepended to gutter (e.g., "│" for tree style, "" for plain)
    pub rail: String,
    /// Optional ANSI color spec applied to the `top`/`mid`/`bottom` glyphs
    /// (the inner error-block frame). Leaves `rail` unstyled so it keeps
    /// matching the surrounding box border.
    #[serde(default)]
    pub color: String,
}

impl Default for ErrorBlockChars {
    fn default() -> Self {
        Self {
            top: "\u{256D}\u{2500}".into(),
            mid: "\u{2502}".into(),
            bottom: "\u{2570}\u{2500}".into(),
            rail: String::new(),
            color: String::new(),
        }
    }
}

/// Per-status icons and colors for report-style command output (`ops deps`),
/// rendered by `ConfigurableTheme::render_report`.
///
/// Defaulted in full so existing themes and `.ops.toml` files need no
/// `[themes.*.report]` block — it is added with `#[serde(default)]` on
/// [`ThemeConfig::report`], and `#[serde(default)]` here lets a user override
/// only the keys they care about. The default glyphs/colors reproduce the
/// long-standing hand-rolled `ops deps` output (`✓ ℹ ⚠ ✘`, green/dim/yellow/red).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ReportTheme {
    /// Icon for a clean / passing row (default `✓`).
    pub icon_ok: String,
    /// Icon for an informational row (default `ℹ`).
    pub icon_info: String,
    /// Icon for a warning row (default `⚠`).
    pub icon_warning: String,
    /// Icon for an error row (default `✘`).
    pub icon_error: String,
    /// ANSI color spec for an ok row's result slot (default `green`).
    pub color_ok: String,
    /// ANSI color spec for an info row's result slot (default `dim`).
    pub color_info: String,
    /// ANSI color spec for a warning row's result slot (default `yellow`).
    pub color_warning: String,
    /// ANSI color spec for an error row's result slot (default `red`).
    pub color_error: String,
    /// ANSI color spec for the report title (default `bold`).
    pub title_color: String,
}

impl Default for ReportTheme {
    fn default() -> Self {
        Self {
            icon_ok: "\u{2713}".into(),      // ✓
            icon_info: "\u{2139}".into(),    // ℹ
            icon_warning: "\u{26a0}".into(), // ⚠
            icon_error: "\u{2718}".into(),   // ✘
            color_ok: "green".into(),
            color_info: "dim".into(),
            color_warning: "yellow".into(),
            color_error: "red".into(),
            title_color: "bold".into(),
        }
    }
}

impl ReportTheme {
    /// Icon glyph for a report status.
    #[must_use]
    pub fn icon(&self, status: ReportStatus) -> &str {
        match status {
            ReportStatus::Ok => &self.icon_ok,
            ReportStatus::Info => &self.icon_info,
            ReportStatus::Warning => &self.icon_warning,
            ReportStatus::Error => &self.icon_error,
        }
    }

    /// ANSI color spec for a report status's result slot.
    #[must_use]
    pub fn color(&self, status: ReportStatus) -> &str {
        match status {
            ReportStatus::Ok => &self.color_ok,
            ReportStatus::Info => &self.color_info,
            ReportStatus::Warning => &self.color_warning,
            ReportStatus::Error => &self.color_error,
        }
    }
}

/// Serializable theme configuration for TOML.
///
/// All properties are customizable. Built-in themes (`classic`, `compact`)
/// are provided as constructors.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    /// Icon for pending steps.
    pub icon_pending: String,
    /// Icon for running steps (often empty, spinner handled by indicatif).
    pub icon_running: String,
    /// Icon for succeeded steps.
    pub icon_succeeded: String,
    /// Icon for failed steps.
    pub icon_failed: String,
    /// Icon for skipped steps.
    pub icon_skipped: String,
    /// Character used for the separator between label and elapsed time.
    pub separator_char: char,
    /// Indent string before the icon on non-running step lines.
    pub step_indent: String,
    /// Indicatif template for running steps.
    pub running_template: String,
    /// Tick characters for the indicatif spinner (last char is steady state).
    pub tick_chars: String,
    /// Columns consumed by the running template outside of `{msg}`.
    pub running_template_overhead: usize,
    /// Style for rendering the plan header.
    #[serde(default)]
    pub plan_header_style: PlanHeaderStyle,
    /// Prefix for the summary line (e.g., "└── " or "→ ").
    pub summary_prefix: String,
    /// Separator string before the summary (e.g., "│" or "").
    pub summary_separator: String,
    /// Box-drawing characters for error detail blocks.
    #[serde(default)]
    pub error_block: ErrorBlockChars,
    /// Optional description for `theme list` output.
    #[serde(default)]
    pub description: Option<String>,
    /// Number of spaces to prepend to all rendered output lines (left margin).
    ///
    /// SEC-33 / TASK-1849: bounded by [`MAX_LEFT_PAD`] at the serde layer.
    /// This value sizes an allocation directly (`" ".repeat(left_pad)` in
    /// `theme::ConfigurableTheme::new`, `runner::display::style`), so an
    /// unbounded `usize` from a repo-supplied `.ops.toml` was a ~400-byte
    /// capacity-overflow panic / OOM-abort primitive. The bound is enforced
    /// during deserialization so an out-of-range value can never be stored,
    /// and again in [`Self::validate`] for programmatically-built configs.
    #[serde(
        default = "default_left_pad",
        deserialize_with = "deserialize_left_pad"
    )]
    pub left_pad: usize,
    /// Optional prefix printed before "Running:" in plain plan headers (e.g. "🚀 ").
    #[serde(default)]
    pub plan_header_prefix: String,
    /// ANSI color spec for the plan header line (e.g. "bold `bright_white`").
    #[serde(default)]
    pub header_color: String,
    /// ANSI color spec for the command label on completed/pending step lines.
    #[serde(default)]
    pub label_color: String,
    /// ANSI color spec for the separator fill between label and duration.
    #[serde(default)]
    pub separator_color: String,
    /// ANSI color spec for the trailing duration.
    #[serde(default)]
    pub duration_color: String,
    /// ANSI color spec for the final summary line ("Done N/N in …").
    #[serde(default)]
    pub summary_color: String,
    /// Overall layout kind (flat or boxed). Defaults to flat for backward compatibility.
    #[serde(default)]
    pub layout_kind: LayoutKind,
    /// Icons and colors for report-style command output (`ops deps`).
    /// Defaulted so existing themes need no `[report]` block.
    #[serde(default)]
    pub report: ReportTheme,
}

const fn default_left_pad() -> usize {
    1
}

/// SEC-33 / TASK-1849: upper bound on [`ThemeConfig::left_pad`].
///
/// `left_pad` is a left margin measured in terminal columns. 1024 is already
/// an order of magnitude past the widest realistic terminal, so the bound
/// cannot reject a legitimate theme, while capping the allocation the value
/// drives at one kibibyte of spaces.
pub const MAX_LEFT_PAD: usize = 1024;

/// SEC-33 / TASK-1849: reject an out-of-range `left_pad` during
/// deserialization, before the value is ever stored on a [`ThemeConfig`].
///
/// serde reports the offending key path (`themes.<name>.left_pad`) around this
/// error, so the operator sees which theme and field is at fault.
fn deserialize_left_pad<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error as _;
    let value = usize::deserialize(deserializer)?;
    if value > MAX_LEFT_PAD {
        return Err(D::Error::custom(format!(
            "left_pad must be at most {MAX_LEFT_PAD} (got {value})"
        )));
    }
    Ok(value)
}

// The `{spinner}` / `{msg}` / `{elapsed}` tokens in `running_template` are
// `indicatif` template placeholders, resolved at render time by the progress
// bar — not Rust format arguments (docs/clippy.md layer 3).
#[allow(clippy::literal_string_with_formatting_args)]
impl ThemeConfig {
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn classic() -> Self {
        Self {
            icon_pending: "\u{25C7}".into(),
            icon_running: String::new(),
            icon_succeeded: "\u{25C6}".into(),
            icon_failed: "\u{2716}".into(),
            icon_skipped: "\u{2298}".into(),
            separator_char: '\u{2500}',
            step_indent: "\u{251C}\u{2500}\u{2500} ".into(),
            running_template: "\u{251C}\u{2500}\u{2500} {spinner:.cyan}{msg} {elapsed:.dim}".into(),
            tick_chars: "|/-\\ ".into(),
            running_template_overhead: 9,
            plan_header_style: PlanHeaderStyle::Tree,
            summary_prefix: "\u{2514}\u{2500}\u{2500} ".into(),
            summary_separator: "\u{2502}".into(),
            error_block: ErrorBlockChars {
                top: "\u{250C}\u{2500}".into(),
                mid: "\u{2502}".into(),
                bottom: "\u{2514}\u{2500}".into(),
                rail: "\u{2502}".into(),
                color: String::new(),
            },
            description: Some("Bold tree-style with box-drawing chars".into()),
            left_pad: 1,
            plan_header_prefix: String::new(),
            header_color: String::new(),
            label_color: String::new(),
            separator_color: String::new(),
            duration_color: String::new(),
            summary_color: String::new(),
            layout_kind: LayoutKind::Flat,
            report: ReportTheme::default(),
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn compact() -> Self {
        Self {
            icon_pending: "\u{25CB}".into(),
            icon_running: String::new(),
            icon_succeeded: "\u{2713}".into(),
            icon_failed: "\u{2717}".into(),
            icon_skipped: "\u{2014}".into(),
            separator_char: '.',
            step_indent: "  ".into(),
            running_template: "  {spinner:.cyan}{msg} {elapsed:.dim}".into(),
            tick_chars: "\u{2801}\u{2802}\u{2804}\u{2840}\u{2880}\u{2820}\u{2810}\u{2808} ".into(),
            running_template_overhead: 7,
            plan_header_style: PlanHeaderStyle::Plain,
            summary_prefix: String::new(),
            summary_separator: String::new(),
            error_block: ErrorBlockChars::default(),
            description: Some("Minimal with dot separators".into()),
            left_pad: 1,
            plan_header_prefix: String::new(),
            header_color: String::new(),
            label_color: String::new(),
            separator_color: String::new(),
            duration_color: String::new(),
            summary_color: String::new(),
            layout_kind: LayoutKind::Flat,
            report: ReportTheme::default(),
        }
    }

    /// SEC-33 / TASK-1849: screen the theme's numeric knobs at config-load
    /// time. `Config::validate` — the one validation `load_config_at` runs —
    /// calls this for every entry in `[themes]`, which until TASK-1849 was the
    /// only config section nothing validated at all.
    ///
    /// The serde layer ([`deserialize_left_pad`]) already rejects an
    /// out-of-range `left_pad` before it can be stored, so for a TOML-sourced
    /// theme this is defence in depth; it is the *only* screen for a
    /// programmatically-built [`ThemeConfig`] (extensions, test-support
    /// constructors), and it is what puts the theme name in the message.
    ///
    /// Audit of the remaining numeric fields, so the next reader does not have
    /// to redo it:
    ///
    /// - `running_template_overhead: usize` — only ever `saturating_sub`ed
    ///   from a `u16` column count; it shrinks a width, never sizes an
    ///   allocation, so any value is inert.
    /// - `OutputConfig::columns` is a `u16` and `stderr_tail_lines` caps a
    ///   ring buffer; both live outside `ThemeConfig` and are bounded by their
    ///   own types.
    ///
    /// `left_pad` is therefore the only allocation lever in this type.
    ///
    /// # Errors
    ///
    /// If `left_pad` exceeds [`MAX_LEFT_PAD`].
    pub fn validate(&self, name: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.left_pad <= MAX_LEFT_PAD,
            "theme '{name}': left_pad must be at most {MAX_LEFT_PAD} (got {})",
            self.left_pad
        );
        Ok(())
    }

    /// Get the icon for a given step status.
    #[must_use]
    pub fn status_icon(&self, status: StepStatus) -> &str {
        match status {
            StepStatus::Pending => &self.icon_pending,
            StepStatus::Running => &self.icon_running,
            StepStatus::Succeeded => &self.icon_succeeded,
            StepStatus::Failed => &self.icon_failed,
            StepStatus::Skipped => &self.icon_skipped,
        }
    }
}
