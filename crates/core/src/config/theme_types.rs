//! Theme configuration types (serializable).
//!
//! These are the data types for theme configuration stored in TOML.
//! The rendering logic that uses these types lives in the theme crate.

use crate::output::StepStatus;
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
}

impl Default for ErrorBlockChars {
    fn default() -> Self {
        Self {
            top: "\u{256D}\u{2500}".into(),
            mid: "\u{2502}".into(),
            bottom: "\u{2570}\u{2500}".into(),
            rail: String::new(),
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
    #[serde(default = "default_left_pad")]
    pub left_pad: usize,
}

fn default_left_pad() -> usize {
    1
}

impl ThemeConfig {
    #[cfg(any(test, feature = "test-support"))]
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
            },
            description: Some("Bold tree-style with box-drawing chars".into()),
            left_pad: 1,
        }
    }

    #[cfg(any(test, feature = "test-support"))]
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
        }
    }

    /// Get the icon for a given step status.
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
