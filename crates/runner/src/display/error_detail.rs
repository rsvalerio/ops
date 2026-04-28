//! Error detail rendering extracted from `ProgressDisplay`.
//!
//! Owns the stderr-tail extraction and theme-driven error block rendering.

use ops_core::output::{tail_lines, ErrorDetail};
use ops_theme as theme;

/// Renders error detail blocks for failed steps.
pub struct ErrorDetailRenderer<'a> {
    theme: &'a dyn theme::StepLineTheme,
    columns: u16,
}

impl<'a> ErrorDetailRenderer<'a> {
    pub fn new(theme: &'a dyn theme::StepLineTheme, columns: u16) -> Self {
        Self { theme, columns }
    }

    pub fn render(&self, message: &str, stderr_tail: &[String]) -> Vec<String> {
        let detail = ErrorDetail::new(message.to_string(), stderr_tail.to_vec());
        self.theme.render_error_detail(&detail, self.columns)
    }

    pub fn extract_stderr_tail(stderr_lines: &[String], max_lines: usize) -> Vec<String> {
        tail_lines(stderr_lines, max_lines).to_vec()
    }
}
