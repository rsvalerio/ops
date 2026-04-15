//! TOML-configurable theme implementation.

use ops_core::output::{ErrorDetail, StepStatus};

use super::render::render_error_block;
use super::step_line_theme::StepLineTheme;
use super::{PlanHeaderStyle, ThemeConfig};

/// A theme backed by a [`ThemeConfig`], implementing [`StepLineTheme`].
pub struct ConfigurableTheme(pub ThemeConfig);

impl StepLineTheme for ConfigurableTheme {
    fn left_pad(&self) -> usize {
        self.0.left_pad
    }

    fn status_icon(&self, status: StepStatus) -> &str {
        self.0.status_icon(status)
    }

    fn separator_char(&self) -> char {
        self.0.separator_char
    }

    fn step_indent(&self) -> &str {
        &self.0.step_indent
    }

    fn summary_prefix(&self) -> &str {
        &self.0.summary_prefix
    }

    fn running_template(&self) -> &str {
        &self.0.running_template
    }

    fn tick_chars(&self) -> &str {
        &self.0.tick_chars
    }

    fn running_template_overhead(&self) -> usize {
        self.0.running_template_overhead
    }

    fn render_plan_header(&self, command_ids: &[String], _columns: u16) -> Vec<String> {
        let pad = self.left_pad_str();
        match self.0.plan_header_style {
            PlanHeaderStyle::Plain => {
                let header = format!("{}Running: {}", pad, command_ids.join(", "));
                vec![String::new(), header, String::new()]
            }
            PlanHeaderStyle::Tree => {
                vec![
                    String::new(),
                    format!("{}┌ Running: {}", pad, command_ids.join(", ")),
                    format!("{}│", pad),
                ]
            }
        }
    }

    fn render_summary_separator(&self, _columns: u16) -> String {
        if self.0.summary_separator.is_empty() {
            String::new()
        } else {
            format!("{}{}", self.left_pad_str(), self.0.summary_separator)
        }
    }

    fn render_error_detail(&self, detail: &ErrorDetail, _columns: u16) -> Vec<String> {
        render_error_block(
            detail,
            self.icon_column_width(),
            &self.0.error_block,
            self.left_pad(),
        )
    }
}
