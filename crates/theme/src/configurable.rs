//! TOML-configurable theme implementation.

use ops_core::output::{display_width, ErrorDetail, StepStatus};

use super::render::render_error_block;
use super::step_line_theme::{format_duration, BoxSnapshot, StepLineTheme};
use super::style::{apply_style, strip_ansi};
use super::{PlanHeaderStyle, ThemeConfig};
use ops_core::config::theme_types::LayoutKind;

/// Columns reserved by the boxed frame on a step line: `│ X  … │` = 7 cells.
const BOX_STEP_RESERVE: u16 = 7;

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

    fn header_color(&self) -> &str {
        &self.0.header_color
    }

    fn label_color(&self) -> &str {
        &self.0.label_color
    }

    fn separator_color(&self) -> &str {
        &self.0.separator_color
    }

    fn duration_color(&self) -> &str {
        &self.0.duration_color
    }

    fn summary_color(&self) -> &str {
        &self.0.summary_color
    }

    fn plan_header_prefix(&self) -> &str {
        &self.0.plan_header_prefix
    }

    fn render_plan_header(&self, command_ids: &[String], _columns: u16) -> Vec<String> {
        let pad = self.left_pad_str();
        let ids = command_ids.join(", ");
        match self.0.plan_header_style {
            PlanHeaderStyle::Plain => {
                let body = format!("{}Running: {}", self.0.plan_header_prefix, ids);
                let colored = apply_style(&body, &self.0.header_color);
                let header = format!("{}{}", pad, colored);
                vec![String::new(), header, String::new()]
            }
            PlanHeaderStyle::Tree => {
                let body = format!("┌ Running: {}", ids);
                let colored = apply_style(&body, &self.0.header_color);
                vec![
                    String::new(),
                    format!("{}{}", pad, colored),
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

    fn step_column_reserve(&self) -> u16 {
        match self.0.layout_kind {
            LayoutKind::Boxed => BOX_STEP_RESERVE,
            LayoutKind::Flat => 0,
        }
    }

    fn box_top_border(&self, snap: BoxSnapshot<'_>) -> Option<String> {
        if !matches!(self.0.layout_kind, LayoutKind::Boxed) {
            return None;
        }
        let verb = if !snap.success {
            "Failing"
        } else if snap.completed == snap.total && snap.total > 0 {
            "Done"
        } else {
            "Running"
        };
        let title = format!(
            " {}{} {}/{} · {} ",
            self.0.plan_header_prefix,
            verb,
            snap.completed,
            snap.total,
            format_duration(snap.elapsed_secs)
        );
        Some(build_horizontal_border(
            &title,
            "╭─",
            "╮",
            snap.columns,
            self.left_pad(),
            &self.0.header_color,
        ))
    }

    fn box_bottom_border(&self, snap: BoxSnapshot<'_>) -> Option<String> {
        if !matches!(self.0.layout_kind, LayoutKind::Boxed) {
            return None;
        }
        let label = if snap.success { "Done" } else { "Failed" };
        let title = format!(
            " {} {}/{} in {} ",
            label,
            snap.completed,
            snap.total,
            format_duration(snap.elapsed_secs)
        );
        Some(build_horizontal_border(
            &title,
            "╰─",
            "╯",
            snap.columns,
            self.left_pad(),
            &self.0.summary_color,
        ))
    }

    fn wrap_step_line(&self, inner: &str, progress_cell: &str, columns: u16) -> String {
        if !matches!(self.0.layout_kind, LayoutKind::Boxed) {
            return inner.to_string();
        }
        let pad = " ".repeat(self.left_pad());
        // Inner visual budget: columns - 2*left_pad - BOX_STEP_RESERVE.
        let outer = columns as usize;
        let frame_overhead = 2 * self.left_pad() + BOX_STEP_RESERVE as usize;
        let inner_budget = outer.saturating_sub(frame_overhead);
        let inner_visible = display_width(&strip_ansi(inner));
        let right_pad = inner_budget.saturating_sub(inner_visible);
        let spaces = " ".repeat(right_pad);
        format!(
            "{pad}│ {cell}  {inner}{spaces} │",
            pad = pad,
            cell = progress_cell,
            inner = inner,
            spaces = spaces,
        )
    }
}

/// Render a horizontal border like `╭─ title ────...───╮`.
///
/// Pads the title with `─` fill to reach `columns`, honoring `left_pad` on the
/// outer margin. `title_color` is applied only to the inline title text so the
/// border itself stays dim/plain.
#[allow(clippy::too_many_arguments)]
fn build_horizontal_border(
    title: &str,
    left_corner: &str,
    right_corner: &str,
    columns: u16,
    left_pad: usize,
    title_color: &str,
) -> String {
    let pad = " ".repeat(left_pad);
    let outer = columns as usize;
    let inner = outer.saturating_sub(2 * left_pad);
    let corner_l_w = display_width(left_corner);
    let corner_r_w = display_width(right_corner);
    let title_w = display_width(title);
    let fill = inner.saturating_sub(corner_l_w + corner_r_w + title_w);
    let fill_str = "─".repeat(fill);
    let colored_title = apply_style(title, title_color);
    format!(
        "{pad}{left_corner}{title}{fill}{right_corner}",
        pad = pad,
        left_corner = left_corner,
        title = colored_title,
        fill = fill_str,
        right_corner = right_corner,
    )
}
