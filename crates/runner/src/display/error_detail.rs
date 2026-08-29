//! Error detail rendering extracted from `ProgressDisplay`.
//!
//! Owns the stderr-tail extraction and theme-driven error block rendering.

use ops_core::output::{tail_lines, ErrorDetail};
use ops_theme::ConfigurableTheme;

/// Renders error detail blocks for failed steps.
pub struct ErrorDetailRenderer<'a> {
    theme: &'a ConfigurableTheme,
    columns: u16,
}

impl<'a> ErrorDetailRenderer<'a> {
    #[must_use]
    pub const fn new(theme: &'a ConfigurableTheme, columns: u16) -> Self {
        Self { theme, columns }
    }

    #[must_use]
    pub fn render(&self, message: &str, stderr_tail: &[String]) -> Vec<String> {
        let detail = ErrorDetail::new(message.to_string(), stderr_tail.to_vec());
        self.theme.render_error_detail(&detail, self.columns)
    }

    /// Error rendering only ever needs the last `max_lines`, so the tail is
    /// stringified once here (small, bounded by `stderr_tail_lines`,
    /// default 5).
    ///
    /// PERF-3 / TASK-1925: the ring now holds owned `Box<str>` lines rather
    /// than `OutputLine` views, so this no longer touches the step's shared
    /// capture buffer at all — see `ProgressState`'s type-level docs.
    #[must_use]
    pub fn extract_stderr_tail(stderr_lines: &[Box<str>], max_lines: usize) -> Vec<String> {
        tail_lines(stderr_lines, max_lines)
            .iter()
            .map(std::string::ToString::to_string)
            .collect()
    }
}
