//! Config passthrough accessors.
//!
//! One- and two-line forwarders over the private `ThemeConfig` held by
//! [`ConfigurableTheme`]. They carry no logic and live in their own file so
//! they do not interleave with the column arithmetic in `configurable.rs`,
//! which most needs careful reading.

use ops_core::output::StepStatus;

use super::ConfigurableTheme;
use crate::step_line_theme::format_duration;

impl ConfigurableTheme {
    /// The configured left margin, in columns.
    #[must_use]
    pub const fn left_pad(&self) -> usize {
        self.config.left_pad
    }

    /// The left margin as a precomputed spaces string.
    #[must_use]
    pub fn left_pad_str(&self) -> &str {
        &self.left_pad_str
    }

    /// The configured icon glyph for `status`.
    #[must_use]
    pub fn status_icon(&self, status: StepStatus) -> &str {
        self.config.status_icon(status)
    }

    /// The configured separator glyph between label and duration.
    #[must_use]
    pub const fn separator_char(&self) -> char {
        self.config.separator_char
    }

    /// The configured indent string for non-running step lines.
    #[must_use]
    pub fn step_indent(&self) -> &str {
        &self.config.step_indent
    }

    /// The configured glyph prefixed to summary lines.
    #[must_use]
    pub fn summary_prefix(&self) -> &str {
        &self.config.summary_prefix
    }

    /// The configured `indicatif` template for running rows.
    #[must_use]
    pub fn running_template(&self) -> &str {
        &self.config.running_template
    }

    /// The configured spinner glyph set, one char per tick.
    #[must_use]
    pub fn tick_chars(&self) -> &str {
        &self.config.tick_chars
    }

    /// The configured column overhead of the running template.
    #[must_use]
    pub const fn running_template_overhead(&self) -> usize {
        self.config.running_template_overhead
    }

    /// The configured SGR color spec for the plan header.
    #[must_use]
    pub fn header_color(&self) -> &str {
        &self.config.header_color
    }

    /// The configured SGR color spec for step labels.
    #[must_use]
    pub fn label_color(&self) -> &str {
        &self.config.label_color
    }

    /// The configured SGR color spec for separators.
    #[must_use]
    pub fn separator_color(&self) -> &str {
        &self.config.separator_color
    }

    /// The configured SGR color spec for durations.
    #[must_use]
    pub fn duration_color(&self) -> &str {
        &self.config.duration_color
    }

    /// The configured SGR color spec for summary lines.
    #[must_use]
    pub fn summary_color(&self) -> &str {
        &self.config.summary_color
    }

    /// The configured prefix text of the plan header.
    #[must_use]
    pub fn plan_header_prefix(&self) -> &str {
        &self.config.plan_header_prefix
    }

    /// Format `secs` as a human-readable duration string.
    #[must_use]
    pub fn format_elapsed(&self, secs: f64) -> String {
        format_duration(secs)
    }
}
