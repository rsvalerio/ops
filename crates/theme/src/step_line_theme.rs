//! Step-line rendering trait and duration formatting.

use ops_core::output::{display_width, ErrorDetail, StepLine, StepStatus, ALL_STATUSES};

use super::render::render_error_block;
use ops_core::config::theme_types::ErrorBlockChars;

/// Format a duration in seconds into a human-friendly string.
///
/// - `< 60s` → `"0.74s"`, `"5.37s"` (two decimal places)
/// - `≥ 60s` → `"2m14s"`, `"4m38s"` (minutes + whole seconds)
/// - `≥ 3600s` → `"1h2m3s"` (hours + minutes + seconds)
pub fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.2}s", secs)
    } else if secs < 3600.0 {
        let mins = (secs / 60.0) as u64;
        let remaining = secs as u64 % 60;
        format!("{}m{}s", mins, remaining)
    } else {
        let hours = (secs / 3600.0) as u64;
        let remaining = secs as u64 % 3600;
        let mins = remaining / 60;
        let secs_part = remaining % 60;
        format!("{}h{}m{}s", hours, mins, secs_part)
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
/// See [`super::ConfigurableTheme`] for the standard TOML-configurable implementation.
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
/// This trait has 15 methods with 12 providing default implementations. The design
/// allows themes to override only what they need:
///
/// - **Core methods (no default)**: `status_icon` — must be implemented
/// - **Layout Methods**: `render`, `render_prefix`, `render_separator` — sensible defaults
/// - **Style Methods**: `separator_char`, `step_indent`, `format_elapsed` — customization
/// - **Progress Methods**: `running_template`, `tick_chars` — spinner control
/// - **Header/Summary**: `render_plan_header`, `render_summary`, `render_summary_separator`, `summary_prefix`
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
/// 3. Method count is stable (15 is acceptable for a rendering trait)
pub trait StepLineTheme: Send + Sync {
    /// Number of spaces to prepend to all rendered output lines. Default: 0.
    fn left_pad(&self) -> usize {
        0
    }

    /// Returns a string of spaces for the left padding.
    fn left_pad_str(&self) -> String {
        " ".repeat(self.left_pad())
    }

    /// Icon string for the given step status.
    fn status_icon(&self, status: StepStatus) -> &str;

    /// Lines to print when a run plan starts: optional upper space, header, then blank before steps.
    /// Default: one blank line (upper space), "Running: id1, id2, ...", then one blank before steps.
    fn render_plan_header(&self, command_ids: &[String], _columns: u16) -> Vec<String> {
        let header = format!("{}Running: {}", self.left_pad_str(), command_ids.join(", "));
        vec![String::new(), header, String::new()]
    }

    /// Character used for the spacer between label and elapsed time. Default: '.'.
    fn separator_char(&self) -> char {
        '.'
    }

    /// Format elapsed seconds for display using human-friendly notation.
    fn format_elapsed(&self, secs: f64) -> String {
        format_duration(secs)
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

    /// Prefix for the summary/footer line (e.g. `"└── "` for tree themes). Default: empty.
    fn summary_prefix(&self) -> &str {
        ""
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
        format!(
            "{}{}{} in {}",
            self.left_pad_str(),
            self.summary_prefix(),
            label,
            format_duration(elapsed_secs)
        )
    }

    /// Render error details as lines displayed below a failed step.
    fn render_error_detail(&self, detail: &ErrorDetail, _columns: u16) -> Vec<String> {
        render_error_block(
            detail,
            self.icon_column_width(),
            &ErrorBlockChars::default(),
            self.left_pad(),
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
        // Running steps get left_pad from the running_template in display.rs;
        // non-running steps (pending/completed) need it here since their template is plain "{msg}".
        let pad = if is_running {
            String::new()
        } else {
            self.left_pad_str()
        };

        if duration_str.is_empty() {
            format!("{}{}{}", pad, prefix, separator)
        } else {
            format!("{}{}{} {}", pad, prefix, separator, duration_str)
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
        let line_budget = columns
            .saturating_sub(template_overhead)
            .saturating_sub(self.left_pad());

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
