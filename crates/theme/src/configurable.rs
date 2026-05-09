//! TOML-configurable theme implementation.

use ops_core::output::{display_width, ErrorDetail, StepLine, StepStatus, ALL_STATUSES};

use super::render::render_error_block;
use super::step_line_theme::{format_duration, BoxSnapshot, StepPrefixParts};
use super::style::{apply_with_prefix, precompute_sgr_prefix, visible_width};
use super::{PlanHeaderStyle, ThemeConfig};
use ops_core::config::theme_types::LayoutKind;

/// Columns reserved by the boxed frame on a step line: `│ X  … │` = 7 cells.
///
/// Layout breakdown (left-to-right):
/// `│` (1) + ` ` (1) + progress cell `X` (1) + `  ` (2) + … + ` ` (1) + `│` (1) = 7.
///
/// The two frame bars are at column 1 and column `columns` — that is, each
/// `BOX_STEP_RESERVE`-based subtraction that uses `- 2` is subtracting exactly
/// those two bars. Keep the constants named so derived offsets don't look
/// like bare arithmetic.
const BOX_STEP_RESERVE: u16 = 7;

/// Number of vertical frame bars consumed by the boxed layout (left `│`
/// and right `│`). Subtracted from `BOX_STEP_RESERVE` when computing the
/// indent that aligns error-block glyphs under the step label column.
const BOX_FRAME_BARS: usize = 2;

/// A theme backed by a [`ThemeConfig`].
///
/// TASK-0747: SGR prefixes are precomputed at construction so the per-step
/// render path avoids repeated spec parsing and allocation.
/// TASK-0748: fields are private; construction goes through [`Self::new`].
/// `#[non_exhaustive]` gates future field additions as non-breaking.
#[non_exhaustive]
pub struct ConfigurableTheme {
    config: ThemeConfig,
    header_prefix: Option<String>,
    summary_prefix: Option<String>,
    label_prefix: Option<String>,
    separator_prefix: Option<String>,
    duration_prefix: Option<String>,
    /// TASK-1035: precomputed `" ".repeat(config.left_pad)` so the per-step
    /// render path doesn't allocate a fresh padding string on every call.
    left_pad_str: String,
}

impl ConfigurableTheme {
    pub fn new(config: ThemeConfig) -> Self {
        let left_pad_str = " ".repeat(config.left_pad);
        Self {
            header_prefix: precompute_sgr_prefix(&config.header_color),
            summary_prefix: precompute_sgr_prefix(&config.summary_color),
            label_prefix: precompute_sgr_prefix(&config.label_color),
            separator_prefix: precompute_sgr_prefix(&config.separator_color),
            duration_prefix: precompute_sgr_prefix(&config.duration_color),
            left_pad_str,
            config,
        }
    }

    pub fn left_pad(&self) -> usize {
        self.config.left_pad
    }

    pub fn left_pad_str(&self) -> &str {
        &self.left_pad_str
    }

    pub fn status_icon(&self, status: StepStatus) -> &str {
        self.config.status_icon(status)
    }

    pub fn separator_char(&self) -> char {
        self.config.separator_char
    }

    pub fn step_indent(&self) -> &str {
        &self.config.step_indent
    }

    pub fn summary_prefix(&self) -> &str {
        &self.config.summary_prefix
    }

    pub fn running_template(&self) -> &str {
        &self.config.running_template
    }

    pub fn tick_chars(&self) -> &str {
        &self.config.tick_chars
    }

    pub fn running_template_overhead(&self) -> usize {
        self.config.running_template_overhead
    }

    pub fn header_color(&self) -> &str {
        &self.config.header_color
    }

    pub fn label_color(&self) -> &str {
        &self.config.label_color
    }

    pub fn separator_color(&self) -> &str {
        &self.config.separator_color
    }

    pub fn duration_color(&self) -> &str {
        &self.config.duration_color
    }

    pub fn summary_color(&self) -> &str {
        &self.config.summary_color
    }

    pub fn plan_header_prefix(&self) -> &str {
        &self.config.plan_header_prefix
    }

    pub fn format_elapsed(&self, secs: f64) -> String {
        format_duration(secs)
    }

    pub fn icon_column_width(&self) -> usize {
        ALL_STATUSES
            .iter()
            .map(|s| display_width(self.status_icon(*s)))
            .max()
            .unwrap_or(0)
    }

    pub fn render_plan_header(&self, command_ids: &[String]) -> Vec<String> {
        let pad = self.left_pad_str();
        let ids = command_ids.join(", ");
        match self.config.plan_header_style {
            PlanHeaderStyle::Plain => {
                let body = format!("{}Running: {}", self.config.plan_header_prefix, ids);
                let colored = apply_with_prefix(&body, self.header_prefix.as_deref());
                let header = format!("{}{}", pad, colored);
                vec![String::new(), header, String::new()]
            }
            PlanHeaderStyle::Tree => {
                let body = format!("┌ Running: {}", ids);
                let colored = apply_with_prefix(&body, self.header_prefix.as_deref());
                vec![
                    String::new(),
                    format!("{}{}", pad, colored),
                    format!("{}│", pad),
                ]
            }
        }
    }

    pub fn render_summary_separator(&self, _columns: u16) -> String {
        if self.config.summary_separator.is_empty() {
            String::new()
        } else {
            format!("{}{}", self.left_pad_str(), self.config.summary_separator)
        }
    }

    pub fn render_error_detail(&self, detail: &ErrorDetail, columns: u16) -> Vec<String> {
        let lines = render_error_block(
            detail,
            self.icon_column_width(),
            &self.config.error_block,
            self.left_pad(),
        );
        if !matches!(self.config.layout_kind, LayoutKind::Boxed) {
            return lines;
        }
        // Boxed: align the mid column under the step label column and close the
        // right frame border. The rail char already matches the frame's left
        // border; we just need extra indent after it so `top`/`mid`/`bottom`
        // land in the same column as the step icon.
        let rail_width = display_width(&self.config.error_block.rail);
        // Subtract the two frame bars (left/right `│`) from the box reserve
        // so `target_gutter` covers only the interior (cell + spacing + step
        // indent) that the error glyph must line up with.
        let target_gutter =
            BOX_STEP_RESERVE as usize - BOX_FRAME_BARS + display_width(self.step_indent());
        let extra_indent = target_gutter.saturating_sub(rail_width + 3);
        let inject = " ".repeat(extra_indent);
        let pad = self.left_pad_str();
        let prefix_with_rail = format!("{}{}", pad, self.config.error_block.rail);

        let outer = columns as usize;
        let right_target = outer.saturating_sub(self.left_pad()).saturating_sub(2);
        lines
            .into_iter()
            .map(|line| {
                let reindented = inject_gutter_indent(&line, &prefix_with_rail, &inject);
                right_pad_with_border(reindented, right_target)
            })
            .collect()
    }

    pub fn step_column_reserve(&self) -> u16 {
        match self.config.layout_kind {
            LayoutKind::Boxed => BOX_STEP_RESERVE,
            LayoutKind::Flat => 0,
        }
    }

    pub fn box_top_border(&self, snap: BoxSnapshot<'_>) -> Option<String> {
        if !matches!(self.config.layout_kind, LayoutKind::Boxed) {
            return None;
        }
        let title = format!(
            " {}Running: {} ",
            self.config.plan_header_prefix,
            snap.command_ids.join(", ")
        );
        Some(build_horizontal_border(BorderArgs {
            title: &title,
            left_corner: "╭─",
            right_corner: "╮",
            columns: snap.columns,
            left_pad: self.left_pad(),
            title_prefix: self.header_prefix.as_deref(),
        }))
    }

    pub fn box_bottom_border(&self, snap: BoxSnapshot<'_>) -> Option<String> {
        if !matches!(self.config.layout_kind, LayoutKind::Boxed) {
            return None;
        }
        // CL-3 / TASK-0771: when a run did not fully succeed, surface the
        // failed/skipped breakdown rather than a single "Done N/M" line — the
        // legacy label conflated terminal-step count with success count.
        let elapsed = format_duration(snap.elapsed_secs);
        let title = if snap.success {
            format!(" Done {}/{} in {} ", snap.completed, snap.total, elapsed)
        } else {
            let succeeded = snap
                .completed
                .saturating_sub(snap.failed)
                .saturating_sub(snap.skipped);
            format!(
                " {} succeeded, {} skipped, {} failed of {} in {} ",
                succeeded, snap.skipped, snap.failed, snap.total, elapsed
            )
        };
        Some(build_horizontal_border(BorderArgs {
            title: &title,
            left_corner: "╰─",
            right_corner: "╯",
            columns: snap.columns,
            left_pad: self.left_pad(),
            title_prefix: self.summary_prefix.as_deref(),
        }))
    }

    pub fn wrap_step_line(&self, inner: &str, progress_cell: &str, columns: u16) -> String {
        if !matches!(self.config.layout_kind, LayoutKind::Boxed) {
            return inner.to_string();
        }
        let pad = self.left_pad_str();
        // Inner visual budget: columns - 2*left_pad - BOX_STEP_RESERVE.
        let outer = columns as usize;
        // Frame overhead = outer margin on both sides + the boxed step reserve.
        // `2 * left_pad` accounts for the left and right outer-pad columns; the
        // reserve itself already includes the two vertical `│` bars.
        let frame_overhead = 2 * self.left_pad() + BOX_STEP_RESERVE as usize;
        let inner_budget = outer.saturating_sub(frame_overhead);
        let inner_visible = visible_width(inner);
        let right_pad = inner_budget.saturating_sub(inner_visible);
        // PERF-3 / TASK-1130: push directly into the result buffer instead of
        // allocating an intermediate `" ".repeat(right_pad)` String per step.
        let mut out = String::with_capacity(
            pad.len() + inner.len() + right_pad + "│   │".len() + progress_cell.len() + 2,
        );
        out.push_str(pad);
        out.push('│');
        out.push(' ');
        out.push_str(progress_cell);
        out.push_str("  ");
        out.push_str(inner);
        for _ in 0..right_pad {
            out.push(' ');
        }
        out.push(' ');
        out.push('│');
        out
    }

    /// DUP-5 / TASK-0354: shared layout for the left portion of a step line.
    /// Both [`render`](Self::render) and [`render_prefix`](Self::render_prefix)
    /// need exactly the same indent / icon / padding triple, and the two
    /// outputs must remain byte-identical in their prefix bytes —
    /// `render_separator` derives layout math from `display_width(plain_prefix)`.
    /// Returning the components separately (rather than re-deriving them in
    /// each caller) makes drift impossible.
    pub fn step_prefix_parts(&self, status: StepStatus, is_running: bool) -> StepPrefixParts<'_> {
        let icon = self.status_icon(status);
        let icon_width = display_width(icon);
        let max_icon_width = self.icon_column_width();
        let (indent, spinner_cols) = if is_running {
            ("", 1usize)
        } else {
            (self.step_indent(), 0usize)
        };
        let pad = " ".repeat(max_icon_width.saturating_sub(icon_width + spinner_cols));
        StepPrefixParts { indent, icon, pad }
    }

    /// Build the left portion of a step line: indent + icon + padding + label.
    pub fn render_prefix(&self, step: &StepLine, is_running: bool) -> String {
        let parts = self.step_prefix_parts(step.status, is_running);
        format!("{}{}{} {}", parts.indent, parts.icon, parts.pad, step.label)
    }

    /// Build the separator (dots/dashes) between label and elapsed time.
    ///
    /// Width budget (left-to-right):
    /// `columns = template_overhead + left_pad + prefix_width + space +
    /// sep_count + space + duration`. We invert that equation to derive
    /// `sep_count`, with a floor of 3 so the separator is always at least
    /// three glyphs wide.
    pub fn render_separator(
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
        let fixed_inside = prefix_width + display_width(duration_str) + leading_space;

        let space_for_sep = line_budget.saturating_sub(fixed_inside);
        const MIN_SEP_GLYPHS: usize = 3;
        let sep_count = space_for_sep.max(MIN_SEP_GLYPHS);
        let sep = self.separator_char();

        // PERF-3 / TASK-1130: build the leading-space + repeated-sep + optional
        // trailing-space directly into a single String, avoiding the intermediate
        // `sep.to_string().repeat(n)` allocation per step render.
        let dots_count = sep_count.saturating_sub(1);
        let trailing_space = duration_str.is_empty();
        let sep_len = sep.len_utf8();
        let mut out = String::with_capacity(1 + dots_count * sep_len + usize::from(trailing_space));
        out.push(' ');
        for _ in 0..dots_count {
            out.push(sep);
        }
        if trailing_space {
            out.push(' ');
        }
        out
    }

    // TASK-0747: render uses precomputed SGR prefixes instead of re-parsing
    // the spec string on every step line.
    pub fn render(&self, step: &StepLine, columns: u16) -> String {
        let is_running = step.status == StepStatus::Running;
        let plain_prefix = self.render_prefix(step, is_running);
        let plain_duration = step
            .elapsed
            .map(|d| self.format_elapsed(d))
            .unwrap_or_default();
        let plain_separator =
            self.render_separator(&plain_prefix, &plain_duration, columns as usize, is_running);
        let pad = if is_running { "" } else { self.left_pad_str() };

        let parts = self.step_prefix_parts(step.status, is_running);
        let colored_label = apply_with_prefix(&step.label, self.label_prefix.as_deref());
        let colored_prefix = format!(
            "{}{}{} {}",
            parts.indent, parts.icon, parts.pad, colored_label
        );

        let colored_separator =
            apply_with_prefix(&plain_separator, self.separator_prefix.as_deref());

        if plain_duration.is_empty() {
            format!("{}{}{}", pad, colored_prefix, colored_separator)
        } else {
            let colored_duration =
                apply_with_prefix(&plain_duration, self.duration_prefix.as_deref());
            format!(
                "{}{}{} {}",
                pad, colored_prefix, colored_separator, colored_duration
            )
        }
    }

    // TASK-0747: render_summary uses precomputed SGR prefix.
    pub fn render_summary(&self, success: bool, elapsed_secs: f64) -> String {
        let label = if success { "Done" } else { "Failed" };
        let body = format!("{} in {}", label, format_duration(elapsed_secs));
        let colored = apply_with_prefix(&body, self.summary_prefix.as_deref());
        format!(
            "{}{}{}",
            self.left_pad_str(),
            self.summary_prefix(),
            colored
        )
    }
}

/// Insert `indent` spaces immediately after the rail prefix on an error-block
/// line so the `top`/`mid`/`bottom` glyphs line up under the step label column.
/// Lines without a rail (empty `rail_prefix`) or that don't start with it are
/// returned unchanged.
fn inject_gutter_indent(line: &str, rail_prefix: &str, indent: &str) -> String {
    if rail_prefix.is_empty() || !line.starts_with(rail_prefix) {
        return line.to_string();
    }
    let (head, tail) = line.split_at(rail_prefix.len());
    format!("{head}{indent}{tail}")
}

/// Right-pad `line` with spaces up to `right_target` visible columns and
/// append the closing ` │` frame border.
fn right_pad_with_border(line: String, right_target: usize) -> String {
    let visible = visible_width(&line);
    let fill = right_target.saturating_sub(visible);
    let spaces = " ".repeat(fill);
    format!("{line}{spaces} │")
}

/// Inputs to [`build_horizontal_border`]. Grouping these as a struct keeps
/// callers legible and avoids the positional-arg smell that
/// `#[allow(clippy::too_many_arguments)]` would otherwise paper over.
struct BorderArgs<'a> {
    title: &'a str,
    left_corner: &'a str,
    right_corner: &'a str,
    columns: u16,
    left_pad: usize,
    title_prefix: Option<&'a str>,
}

/// Render a horizontal border like `╭─ title ────...───╮`.
///
/// Pads the title with `─` fill to reach `columns`, honoring `left_pad` on the
/// outer margin. `title_prefix` is the precomputed SGR prefix applied only to
/// the inline title text so the border itself stays dim/plain.
fn build_horizontal_border(args: BorderArgs<'_>) -> String {
    let BorderArgs {
        title,
        left_corner,
        right_corner,
        columns,
        left_pad,
        title_prefix,
    } = args;
    let pad = " ".repeat(left_pad);
    let outer = columns as usize;
    let inner = outer.saturating_sub(2 * left_pad);
    let corner_l_w = display_width(left_corner);
    let corner_r_w = display_width(right_corner);
    let title_w = display_width(title);
    let fill = inner.saturating_sub(corner_l_w + corner_r_w + title_w);
    let fill_str = "─".repeat(fill);
    let colored_title = apply_with_prefix(title, title_prefix);
    format!("{pad}{left_corner}{colored_title}{fill_str}{right_corner}")
}
