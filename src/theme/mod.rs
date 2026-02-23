//! Theme types and step-line rendering.
//!
//! [`ThemeConfig`] is the serializable theme definition (TOML-compatible).
//! [`ConfigurableTheme`] wraps a `ThemeConfig` and implements [`StepLineTheme`]
//! for rendering step lines and error details.

use crate::output::{display_width, ErrorDetail, StepLine, StepStatus, ALL_STATUSES};
use indexmap::IndexMap;
use serde::Deserialize;

/// Style for rendering the plan header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanHeaderStyle {
    /// Plain header: "Running: cmd1, cmd2"
    #[default]
    Plain,
    /// Tree-style header with box-drawing chars: "┌ Running: cmd1, cmd2" + "│"
    Tree,
}

/// Box-drawing characters for error detail blocks.
#[derive(Debug, Clone, Deserialize)]
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
            top: "╭─".into(),
            mid: "│".into(),
            bottom: "╰─".into(),
            rail: String::new(),
        }
    }
}

/// Serializable theme configuration for TOML.
///
/// All properties are customizable. Built-in themes (`classic`, `compact`)
/// are provided as constructors.
#[derive(Debug, Clone, Deserialize)]
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
    #[allow(dead_code)]
    pub description: Option<String>,
}

impl ThemeConfig {
    #[cfg(test)]
    pub fn classic() -> Self {
        Self {
            icon_pending: "◇".into(),
            icon_running: String::new(),
            icon_succeeded: "◆".into(),
            icon_failed: "✖".into(),
            icon_skipped: "⊘".into(),
            separator_char: '─',
            step_indent: "├── ".into(),
            running_template: "├── {spinner:.cyan}{msg} {elapsed:.dim}".into(),
            tick_chars: "|/-\\ ".into(),
            running_template_overhead: 9,
            plan_header_style: PlanHeaderStyle::Tree,
            summary_prefix: "└── ".into(),
            summary_separator: "│".into(),
            error_block: ErrorBlockChars {
                top: "┌─".into(),
                mid: "│".into(),
                bottom: "└─".into(),
                rail: "│".into(),
            },
            description: Some("Bold tree-style with box-drawing chars".into()),
        }
    }

    #[cfg(test)]
    pub fn compact() -> Self {
        Self {
            icon_pending: "○".into(),
            icon_running: String::new(),
            icon_succeeded: "✓".into(),
            icon_failed: "✗".into(),
            icon_skipped: "—".into(),
            separator_char: '.',
            step_indent: "  ".into(),
            running_template: "  {spinner:.cyan}{msg} {elapsed:.dim}".into(),
            tick_chars: "⠁⠂⠄⡀⢀⠠⠐⠈ ".into(),
            running_template_overhead: 7,
            plan_header_style: PlanHeaderStyle::Plain,
            summary_prefix: String::new(),
            summary_separator: String::new(),
            error_block: ErrorBlockChars::default(),
            description: Some("Minimal with dot separators".into()),
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

/// A theme backed by a [`ThemeConfig`], implementing [`StepLineTheme`].
pub struct ConfigurableTheme(pub ThemeConfig);

impl StepLineTheme for ConfigurableTheme {
    fn status_icon(&self, status: StepStatus) -> &str {
        self.0.status_icon(status)
    }

    fn separator_char(&self) -> char {
        self.0.separator_char
    }

    fn step_indent(&self) -> &str {
        &self.0.step_indent
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
        match self.0.plan_header_style {
            PlanHeaderStyle::Plain => {
                let header = format!("Running: {}", command_ids.join(", "));
                vec![String::new(), header, String::new()]
            }
            PlanHeaderStyle::Tree => {
                vec![
                    String::new(),
                    format!("┌ Running: {}", command_ids.join(", ")),
                    "│".to_string(),
                ]
            }
        }
    }

    fn render_summary(&self, success: bool, elapsed_secs: f64) -> String {
        let label = if success { "Done" } else { "Failed" };
        format!("{}{} in {:.2}s", self.0.summary_prefix, label, elapsed_secs)
    }

    fn render_summary_separator(&self, _columns: u16) -> String {
        self.0.summary_separator.clone()
    }

    fn render_error_detail(&self, detail: &ErrorDetail, _columns: u16) -> Vec<String> {
        render_error_block(detail, self.icon_column_width(), &self.0.error_block)
    }
}

/// Theme for rendering step lines (icon, separator, elapsed format).
///
/// A theme controls the visual appearance of step lines in the CLI output:
/// - Icons for each status (pending, running, succeeded, failed, skipped)
/// - Separator characters between label and elapsed time
/// - Indentation and layout
/// - Progress bar styling for running steps
/// - Plan header and summary formatting
/// - Error detail block rendering
///
/// Implementations must be `Send + Sync` for use across threads.
/// See [`ConfigurableTheme`] for the standard TOML-configurable implementation.
///
/// # Example
///
/// ```ignore
/// struct MyTheme;
///
/// impl StepLineTheme for MyTheme {
///     fn status_icon(&self, status: StepStatus) -> &str {
///         match status {
///             StepStatus::Succeeded => "✓",
///             StepStatus::Failed => "✗",
///             // ...
///         }
///     }
/// }
/// ```
///
/// # Architecture (CQ-016)
///
/// This trait has 14 methods with 11 providing default implementations. The design
/// allows themes to override only what they need:
///
/// - **Core methods (no default)**: `status_icon` — must be implemented
/// - **Layout Methods**: `render`, `render_prefix`, `render_separator` — sensible defaults
/// - **Style Methods**: `separator_char`, `step_indent`, `format_elapsed` — customization
/// - **Progress Methods**: `running_template`, `tick_chars` — spinner control
/// - **Header/Summary**: `render_plan_header`, `render_summary`, `render_summary_separator`
/// - **Error Display**: `render_error_detail`
///
/// Alternative designs considered:
/// - **Split traits**: `CoreTheme` + `ExtendedTheme` — adds complexity without benefit
/// - **Builder pattern**: `ThemeBuilder` with method chaining — more verbose
/// - **Composition**: `Theme { icons: IconConfig, layout: LayoutConfig }` — loses trait flexibility
///
/// The current design is kept because:
/// 1. Default implementations cover 80% of use cases
/// 2. Single trait is easier to implement for custom themes
/// 3. Method count is stable (14 is acceptable for a rendering trait)
pub trait StepLineTheme: Send + Sync {
    /// Icon string for the given step status.
    fn status_icon(&self, status: StepStatus) -> &str;

    /// Lines to print when a run plan starts: optional upper space, header, then blank before steps.
    /// Default: one blank line (upper space), "Running: id1, id2, ...", then one blank before steps.
    fn render_plan_header(&self, command_ids: &[String], _columns: u16) -> Vec<String> {
        let header = format!("Running: {}", command_ids.join(", "));
        vec![String::new(), header, String::new()]
    }

    /// Character used for the spacer between label and elapsed time. Default: '.'.
    fn separator_char(&self) -> char {
        '.'
    }

    /// Format elapsed seconds for display. Default: "{:.2}s".
    fn format_elapsed(&self, secs: f64) -> String {
        format!("{:.2}s", secs)
    }

    /// Maximum display width across all status icons, used to pad narrower icons
    /// so the label column stays aligned.
    ///
    /// Note: `ALL_STATUSES` is a static constant containing all 5 status variants,
    /// so `max()` always returns a value.
    fn icon_column_width(&self) -> usize {
        const _: () = assert!(!ALL_STATUSES.is_empty(), "ALL_STATUSES must not be empty");
        ALL_STATUSES
            .iter()
            .map(|s| display_width(self.status_icon(*s)))
            .max()
            .expect("ALL_STATUSES is guaranteed non-empty by const assert")
    }

    /// Render a separator line for the summary section.
    fn render_summary_separator(&self, _columns: u16) -> String {
        String::new()
    }

    /// Indent before the icon on non-running step lines. Default: `"  "` (2 spaces).
    fn step_indent(&self) -> &str {
        "  "
    }

    /// Indicatif template for running steps. Default matches the compact style.
    fn running_template(&self) -> &str {
        "  {spinner:.cyan}{msg} {elapsed:.dim}"
    }

    /// Tick characters for the indicatif spinner. The last character is the
    /// "steady" state shown when the spinner is not ticking.
    fn tick_chars(&self) -> &str {
        "⠁⠂⠄⡀⢀⠠⠐⠈ "
    }

    /// Columns consumed by the running template outside of `{msg}`.
    fn running_template_overhead(&self) -> usize {
        7
    }

    /// Render the final summary line.
    fn render_summary(&self, success: bool, elapsed_secs: f64) -> String {
        let label = if success { "Done" } else { "Failed" };
        format!("{} in {:.2}s", label, elapsed_secs)
    }

    /// Render error details as lines displayed below a failed step.
    fn render_error_detail(&self, detail: &ErrorDetail, _columns: u16) -> Vec<String> {
        render_error_block(
            detail,
            self.icon_column_width(),
            &ErrorBlockChars::default(),
        )
    }

    /// Render a full step line: "  {icon} {label} {separator...} {elapsed}".
    fn render(&self, step: &StepLine, columns: u16) -> String {
        let is_running = step.status == StepStatus::Running;
        let prefix = self.render_prefix(step, is_running);
        let duration_str = step
            .elapsed
            .map(|d| self.format_elapsed(d))
            .unwrap_or_default();
        let separator = self.render_separator(&prefix, &duration_str, columns as usize, is_running);

        if duration_str.is_empty() {
            format!("{}{}", prefix, separator)
        } else {
            format!("{}{} {}", prefix, separator, duration_str)
        }
    }

    /// Build the left portion of a step line: indent + icon + padding + label.
    fn render_prefix(&self, step: &StepLine, is_running: bool) -> String {
        let icon = self.status_icon(step.status);
        let icon_width = display_width(icon);
        let max_icon_width = self.icon_column_width();
        let (indent, spinner_cols) = if is_running {
            ("", 1usize)
        } else {
            (self.step_indent(), 0usize)
        };
        let pad = " ".repeat(max_icon_width.saturating_sub(icon_width + spinner_cols));
        format!("{}{}{} {}", indent, icon, pad, step.label)
    }

    /// Build the separator (dots/dashes) between label and elapsed time.
    fn render_separator(
        &self,
        prefix: &str,
        duration_str: &str,
        columns: usize,
        is_running: bool,
    ) -> String {
        let template_overhead = if is_running {
            self.running_template_overhead()
        } else {
            0
        };
        let line_budget = columns.saturating_sub(template_overhead);

        let prefix_width = display_width(prefix);
        let sep = self.separator_char();
        let space_for_sep = line_budget.saturating_sub(prefix_width + duration_str.len() + 1);
        let sep_count = space_for_sep.max(3);

        if duration_str.is_empty() {
            let dots = sep.to_string().repeat(sep_count.saturating_sub(1));
            format!(" {}{}", dots, ' ')
        } else {
            let dots = sep.to_string().repeat(sep_count);
            format!(" {}", dots)
        }
    }
}

/// Shared helper for rendering error detail blocks with configurable box-drawing characters.
fn render_error_block(
    detail: &ErrorDetail,
    icon_column_width: usize,
    chars: &ErrorBlockChars,
) -> Vec<String> {
    if detail.message.is_empty() && detail.stderr_tail.is_empty() {
        return Vec::new();
    }
    let gutter = if chars.rail.is_empty() {
        " ".repeat(icon_column_width + 3)
    } else {
        format!("{}   ", chars.rail)
    };
    let mut lines = Vec::new();
    lines.push(format!("{}{}", gutter, chars.top));
    if !detail.message.is_empty() {
        lines.push(format!("{}{} {}", gutter, chars.mid, detail.message));
    }
    if !detail.stderr_tail.is_empty() {
        lines.push(format!(
            "{}{} stderr (last {} lines):",
            gutter,
            chars.mid,
            detail.stderr_tail.len()
        ));
        for stderr_line in &detail.stderr_tail {
            lines.push(format!("{}{}   {}", gutter, chars.mid, stderr_line));
        }
    }
    lines.push(format!("{}{}", gutter, chars.bottom));
    lines
}

/// Error type for theme resolution failures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ThemeError {
    #[error("Theme not found: {0}")]
    NotFound(String),
}

/// Resolve a theme name to a concrete [`StepLineTheme`] implementation.
///
/// Looks up the theme in the provided IndexMap (includes built-in themes from default config).
pub fn resolve_theme(
    name: &str,
    themes: &IndexMap<String, ThemeConfig>,
) -> Result<Box<dyn StepLineTheme>, ThemeError> {
    themes
        .get(name)
        .map(|tc| Box::new(ConfigurableTheme(tc.clone())) as Box<dyn StepLineTheme>)
        .ok_or_else(|| ThemeError::NotFound(name.to_string()))
}

/// List all available theme names.
/// Note: Used by tests and available for programmatic access.
#[allow(dead_code)]
pub fn list_theme_names(themes: &IndexMap<String, ThemeConfig>) -> Vec<String> {
    themes.keys().cloned().collect()
}

#[cfg(test)]
mod tests;
