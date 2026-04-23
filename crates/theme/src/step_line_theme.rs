//! Step-line rendering trait and duration formatting.

use ops_core::output::{display_width, ErrorDetail, StepLine, StepStatus, ALL_STATUSES};

use super::render::render_error_block;
use super::style::apply_style;
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

/// Snapshot of run-plan progress passed to the boxed layout border methods.
///
/// Grouping these fields into a struct keeps the trait method signatures narrow
/// (clippy `too_many_arguments`) and lets the caller compute each value once.
#[derive(Debug, Clone, Copy)]
pub struct BoxSnapshot<'a> {
    /// Number of steps completed so far.
    pub completed: usize,
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

impl<'a> BoxSnapshot<'a> {
    /// Construct a snapshot from raw fields. `command_ids` defaults to empty.
    pub fn new(
        completed: usize,
        total: usize,
        elapsed_secs: f64,
        success: bool,
        columns: u16,
    ) -> Self {
        Self {
            completed,
            total,
            elapsed_secs,
            success,
            columns,
            command_ids: &[],
        }
    }

    /// Attach command IDs to the snapshot (builder style).
    pub fn with_command_ids(mut self, command_ids: &'a [String]) -> Self {
        self.command_ids = command_ids;
        self
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
/// ```text
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
/// # Architecture (CQ-016 / FN-3)
///
/// This trait exposes 22 methods across six concerns. Only `status_icon` is
/// required; all other methods carry defaults that read values off the
/// [`ThemeConfig`](super::ThemeConfig)-backed [`super::ConfigurableTheme`], so
/// a custom theme can override only what it actually customises.
///
/// The methods group as follows (the declarations below follow the same order):
///
/// 1. **Padding / indent**: `left_pad`, `left_pad_str`, `step_indent`
/// 2. **Icons**: `status_icon`, `icon_column_width`
/// 3. **Colors**: `header_color`, `label_color`, `separator_color`,
///    `duration_color`, `summary_color`
/// 4. **Header / summary**: `plan_header_prefix`, `render_plan_header`,
///    `render_summary_separator`, `summary_prefix`, `render_summary`
/// 5. **Progress / running**: `running_template`, `tick_chars`,
///    `running_template_overhead`
/// 6. **Step rendering**: `render`, `render_prefix`, `render_separator`,
///    `separator_char`, `format_elapsed`
/// 7. **Boxed layout**: `box_top_border`, `box_bottom_border`,
///    `step_column_reserve`, `wrap_step_line`
/// 8. **Error detail**: `render_error_detail`
///
/// Alternative designs considered and deferred:
/// - **Split traits** (`StepLineTheme` + `BoxedLayoutTheme` + `ErrorBlockTheme`
///   with blanket impls) — would better respect ISP but would fragment the
///   theme surface across imports for no concrete caller benefit today.
/// - **Concrete-struct defaults** (move the "look up value on `ThemeConfig`"
///   defaults into `ConfigurableTheme` and shrink the trait to the 3–4
///   methods that genuinely vary) — the larger mechanical change.
///
/// The present shape is intentional: `ConfigurableTheme` covers every built-in
/// theme via TOML, so the many defaulted methods don't cost real
/// implementations. If a second non-configurable theme appears, revisit.
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

    /// ANSI color spec for the plan header text. Empty = no color.
    fn header_color(&self) -> &str {
        ""
    }

    /// ANSI color spec for the command label on completed/pending step lines.
    fn label_color(&self) -> &str {
        ""
    }

    /// ANSI color spec for the separator fill between label and duration.
    fn separator_color(&self) -> &str {
        ""
    }

    /// ANSI color spec for the trailing duration on step lines.
    fn duration_color(&self) -> &str {
        ""
    }

    /// ANSI color spec for the final summary line.
    fn summary_color(&self) -> &str {
        ""
    }

    /// Optional prefix printed before "Running:" in plain plan headers.
    fn plan_header_prefix(&self) -> &str {
        ""
    }

    /// Lines to print when a run plan starts: optional upper space, header, then blank before steps.
    /// Default: one blank line (upper space), "Running: id1, id2, ...", then one blank before steps.
    ///
    /// Note: the default implementation intentionally does not wrap or
    /// truncate by terminal width. Callers that need width-aware wrapping
    /// should override this method; the trait used to carry an unused
    /// `columns` parameter which was removed after [`TASK-0281`].
    fn render_plan_header(&self, command_ids: &[String]) -> Vec<String> {
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
        let body = format!("{} in {}", label, format_duration(elapsed_secs));
        let colored = apply_style(&body, self.summary_color());
        format!(
            "{}{}{}",
            self.left_pad_str(),
            self.summary_prefix(),
            colored
        )
    }

    /// Optional top border of a boxed layout, including a live summary.
    ///
    /// Returning `Some` opts in to the "boxed" layout: `ProgressDisplay` will render
    /// this string as the header bar (instead of the classic plan header), update it
    /// each time a step completes, and call [`Self::wrap_step_line`] for each step.
    /// Returning `None` preserves the classic flat layout.
    fn box_top_border(&self, _snap: BoxSnapshot<'_>) -> Option<String> {
        None
    }

    /// Optional bottom border of a boxed layout, rendered on run finish.
    fn box_bottom_border(&self, _snap: BoxSnapshot<'_>) -> Option<String> {
        None
    }

    /// Number of terminal columns reserved by the frame on a step line, subtracted
    /// from the columns budget before calling [`Self::render`]. Default: 0.
    ///
    /// Boxed themes override this to reserve room for `│ cell  … │`.
    fn step_column_reserve(&self) -> u16 {
        0
    }

    /// Wrap a rendered step line in the boxed frame, with a vertical-progress cell
    /// on the left. Default: identity (returns `inner` unchanged).
    ///
    /// `progress_cell` is a single-width glyph representing the overall plan progress
    /// for this row (e.g. `"█"` done, `"▓"` current, `"░"` pending).
    fn wrap_step_line(&self, inner: &str, _progress_cell: &str, _columns: u16) -> String {
        inner.to_string()
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
        // Plain prefix drives layout math (width calc must not include ANSI escapes).
        let plain_prefix = self.render_prefix(step, is_running);
        let plain_duration = step
            .elapsed
            .map(|d| self.format_elapsed(d))
            .unwrap_or_default();
        let plain_separator =
            self.render_separator(&plain_prefix, &plain_duration, columns as usize, is_running);
        // Running steps get left_pad from the running_template in display.rs;
        // non-running steps (pending/completed) need it here since their template is plain "{msg}".
        let pad = if is_running {
            String::new()
        } else {
            self.left_pad_str()
        };

        // Re-emit prefix with the label segment colored (icon + padding stay plain).
        let icon = self.status_icon(step.status);
        let icon_width = display_width(icon);
        let max_icon_width = self.icon_column_width();
        let (indent, spinner_cols) = if is_running {
            ("", 1usize)
        } else {
            (self.step_indent(), 0usize)
        };
        let icon_pad = " ".repeat(max_icon_width.saturating_sub(icon_width + spinner_cols));
        let colored_label = apply_style(&step.label, self.label_color());
        let colored_prefix = format!("{}{}{} {}", indent, icon, icon_pad, colored_label);

        let colored_separator = apply_style(&plain_separator, self.separator_color());

        if plain_duration.is_empty() {
            format!("{}{}{}", pad, colored_prefix, colored_separator)
        } else {
            let colored_duration = apply_style(&plain_duration, self.duration_color());
            format!(
                "{}{}{} {}",
                pad, colored_prefix, colored_separator, colored_duration
            )
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
    ///
    /// Width budget (left-to-right):
    /// `columns = template_overhead + left_pad + prefix_width + space +
    /// sep_count + space + duration`. We invert that equation to derive
    /// `sep_count`, with a floor of 3 so the separator is always at least
    /// three glyphs wide.
    fn render_separator(
        &self,
        prefix: &str,
        duration_str: &str,
        columns: usize,
        is_running: bool,
    ) -> String {
        // Reservations taken out of the total `columns` budget before we can
        // spend anything on the separator itself.
        let template_overhead = if is_running {
            self.running_template_overhead()
        } else {
            0
        };
        let reserved_chrome = template_overhead + self.left_pad();
        let line_budget = columns.saturating_sub(reserved_chrome);

        // Fixed costs inside `line_budget`: the label prefix, the duration
        // (when present), and one leading space before the separator.
        let prefix_width = display_width(prefix);
        let leading_space = 1usize;
        let fixed_inside = prefix_width + duration_str.len() + leading_space;

        let space_for_sep = line_budget.saturating_sub(fixed_inside);
        const MIN_SEP_GLYPHS: usize = 3;
        let sep_count = space_for_sep.max(MIN_SEP_GLYPHS);
        let sep = self.separator_char();

        if duration_str.is_empty() {
            let dots = sep.to_string().repeat(sep_count.saturating_sub(1));
            format!(" {}{}", dots, ' ')
        } else {
            let dots = sep.to_string().repeat(sep_count.saturating_sub(1));
            format!(" {}", dots)
        }
    }
}
