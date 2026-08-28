//! Config passthrough accessors.
//!
//! ARCH-1 / TASK-1981: these one- and two-line forwarders exist because the
//! `ThemeConfig` fields went private in TASK-0748. They carry no logic but
//! dominated the public surface of `configurable.rs`, interleaved with the
//! column arithmetic that most needs careful reading. They are the same
//! methods on the same type — only their file changed.

use ops_core::output::StepStatus;

use super::ConfigurableTheme;
use crate::step_line_theme::format_duration;

impl ConfigurableTheme {
    #[must_use]
    pub const fn left_pad(&self) -> usize {
        self.config.left_pad
    }

    #[must_use]
    pub fn left_pad_str(&self) -> &str {
        &self.left_pad_str
    }

    #[must_use]
    pub fn status_icon(&self, status: StepStatus) -> &str {
        self.config.status_icon(status)
    }

    #[must_use]
    pub const fn separator_char(&self) -> char {
        self.config.separator_char
    }

    #[must_use]
    pub fn step_indent(&self) -> &str {
        &self.config.step_indent
    }

    #[must_use]
    pub fn summary_prefix(&self) -> &str {
        &self.config.summary_prefix
    }

    #[must_use]
    pub fn running_template(&self) -> &str {
        &self.config.running_template
    }

    #[must_use]
    pub fn tick_chars(&self) -> &str {
        &self.config.tick_chars
    }

    #[must_use]
    pub const fn running_template_overhead(&self) -> usize {
        self.config.running_template_overhead
    }

    #[must_use]
    pub fn header_color(&self) -> &str {
        &self.config.header_color
    }

    #[must_use]
    pub fn label_color(&self) -> &str {
        &self.config.label_color
    }

    #[must_use]
    pub fn separator_color(&self) -> &str {
        &self.config.separator_color
    }

    #[must_use]
    pub fn duration_color(&self) -> &str {
        &self.config.duration_color
    }

    #[must_use]
    pub fn summary_color(&self) -> &str {
        &self.config.summary_color
    }

    #[must_use]
    pub fn plan_header_prefix(&self) -> &str {
        &self.config.plan_header_prefix
    }

    #[must_use]
    pub fn format_elapsed(&self, secs: f64) -> String {
        format_duration(secs)
    }
}
