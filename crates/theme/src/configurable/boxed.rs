//! Boxed-layout frame geometry.
//!
//! ARCH-1 / TASK-1981: split out of `configurable.rs`, which had grown past
//! the module red flag with four unrelated concerns behind one type. This is
//! the column arithmetic — frame reserves, borders, the wrap helpers and the
//! error-block re-indent — kept together so it can be reviewed on its own
//! rather than interleaved with config getters and the report path.
//!
//! The width-measurement and truncation policies these helpers follow are
//! documented on the parent module.

use ops_core::config::theme_types::LayoutKind;
use ops_core::output::ErrorDetail;

use super::ConfigurableTheme;
use crate::render::render_error_block;
use crate::step_line_theme::{format_duration, BoxSnapshot};
use crate::style::{apply_with_prefix, truncate_to_width, visible_width};

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

/// FN-1 / TASK-1192: columns the rail prefix occupies *after* the rail
/// glyph itself when computing the boxed error-block gutter offset.
///
/// The rail glyph (`config.error_block.rail`) is rendered immediately
/// after the left frame; the boxed layout then reserves three additional
/// columns before the error icon: the post-rail space, the step-cell
/// alignment column, and the inter-column gutter. Naming the constant
/// keeps the offset traceable instead of looking like a bare `+ 3`.
const BOX_RAIL_PREFIX_PADDING: usize = 3;

impl ConfigurableTheme {
    #[must_use]
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
        let extra_indent = self.boxed_error_indent_columns();
        let inject = " ".repeat(extra_indent);
        let pad = self.left_pad_str();
        let prefix_with_rail = format!("{}{}", pad, self.config.error_block.rail);

        let outer = usize::from(columns);
        let right_target = outer.saturating_sub(self.left_pad()).saturating_sub(2);
        lines
            .into_iter()
            .map(|line| {
                let reindented = inject_gutter_indent(&line, &prefix_with_rail, &inject);
                right_pad_with_border(&reindented, right_target)
            })
            .collect()
    }

    /// FN-1 / TASK-1192: columns of extra indent to inject after the rail
    /// glyph so boxed-layout `top`/`mid`/`bottom` lines align under the
    /// step icon column.
    ///
    /// Subtract the two frame bars (`│ … │`) from `BOX_STEP_RESERVE` so
    /// `target_gutter` covers only the interior (cell + spacing + step
    /// indent) that the error glyph must line up with, then back off by
    /// the rail width plus the named [`BOX_RAIL_PREFIX_PADDING`] columns
    /// the rail prefix already occupies.
    fn boxed_error_indent_columns(&self) -> usize {
        let rail_width = visible_width(&self.config.error_block.rail);
        let target_gutter = usize::from(BOX_STEP_RESERVE)
            .saturating_sub(BOX_FRAME_BARS)
            .saturating_add(visible_width(self.step_indent()));
        target_gutter.saturating_sub(rail_width.saturating_add(BOX_RAIL_PREFIX_PADDING))
    }

    #[must_use]
    pub const fn step_column_reserve(&self) -> u16 {
        match self.config.layout_kind {
            LayoutKind::Boxed => BOX_STEP_RESERVE,
            LayoutKind::Flat => 0,
        }
    }

    #[must_use]
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

    #[must_use]
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

    #[must_use]
    pub fn wrap_step_line(&self, inner: &str, progress_cell: &str, columns: u16) -> String {
        if !matches!(self.config.layout_kind, LayoutKind::Boxed) {
            return inner.to_string();
        }
        let pad = self.left_pad_str();
        // Inner visual budget: columns - 2*left_pad - BOX_STEP_RESERVE.
        let outer = usize::from(columns);
        // Frame overhead = outer margin on both sides + the boxed step reserve.
        // `2 * left_pad` accounts for the left and right outer-pad columns; the
        // reserve itself already includes the two vertical `│` bars.
        let frame_overhead = self
            .left_pad()
            .saturating_mul(2)
            .saturating_add(usize::from(BOX_STEP_RESERVE));
        let inner_budget = outer.saturating_sub(frame_overhead);
        // CL-3 / TASK-1969: clamp the content to the interior budget so the
        // closing bar lands in the same column as the borders even when the
        // caller hands us a line wider than the frame.
        let inner = truncate_to_width(inner, inner_budget);
        let inner_visible = visible_width(&inner);
        let right_pad = inner_budget.saturating_sub(inner_visible);
        // PERF-3 / TASK-1130: push directly into the result buffer instead of
        // allocating an intermediate `" ".repeat(right_pad)` String per step.
        let mut out = String::with_capacity(
            pad.len()
                .saturating_add(inner.len())
                .saturating_add(right_pad)
                .saturating_add("│   │".len())
                .saturating_add(progress_cell.len())
                .saturating_add(2),
        );
        out.push_str(pad);
        out.push('│');
        out.push(' ');
        out.push_str(progress_cell);
        out.push_str("  ");
        out.push_str(&inner);
        for _ in 0..right_pad {
            out.push(' ');
        }
        out.push(' ');
        out.push('│');
        out
    }

    /// Wrap an already-formatted content line in the boxed `│ … │` frame,
    /// right-padding so the closing bar aligns with [`wrap_step_line`]'s. Used
    /// for report detail/continuation lines, which carry their own indentation
    /// and so don't take the progress-cell column.
    pub(super) fn wrap_box_content(&self, inner: &str, columns: u16) -> String {
        let pad = self.left_pad_str();
        // Interior between the two `│` bars, minus the leading and trailing
        // interior spaces — the content area whose right edge must line up with
        // the right bar that `wrap_step_line` emits at `columns - left_pad`.
        let content_area = usize::from(columns)
            .saturating_sub(self.left_pad().saturating_mul(2))
            .saturating_sub(4);
        // CL-3 / TASK-1969: report detail lines are arbitrary-length strings
        // the report producer supplies; clamp them to the content area so the
        // closing bar aligns with `wrap_step_line`'s.
        let inner = truncate_to_width(inner, content_area);
        let right_pad = content_area.saturating_sub(visible_width(&inner));
        format!("{pad}│ {inner}{} │", " ".repeat(right_pad))
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
fn right_pad_with_border(line: &str, right_target: usize) -> String {
    // CL-3 / TASK-1969 + SEC-21 / TASK-1965: the error block carries
    // sanitised subprocess stderr, which is both arbitrary-length and wider
    // than its source once escapes are rendered as `\xNN`. Clamp before
    // padding so the closing bar always lands at `right_target`.
    let line = truncate_to_width(line, right_target);
    let visible = visible_width(&line);
    let fill = right_target.saturating_sub(visible);
    let spaces = " ".repeat(fill);
    format!("{line}{spaces} │")
}

/// Inputs to [`build_horizontal_border`]. Grouping these as a struct keeps
/// callers legible and avoids the positional-arg smell that
/// `#[allow(clippy::too_many_arguments)]` would otherwise paper over.
#[derive(Clone, Copy)]
pub(super) struct BorderArgs<'a> {
    pub(super) title: &'a str,
    pub(super) left_corner: &'a str,
    pub(super) right_corner: &'a str,
    pub(super) columns: u16,
    pub(super) left_pad: usize,
    pub(super) title_prefix: Option<&'a str>,
}

/// Render a horizontal border like `╭─ title ────...───╮`.
///
/// Pads the title with `─` fill to reach `columns`, honoring `left_pad` on the
/// outer margin. The title is first clamped to the interior left over once
/// both corners are accounted for, so the rendered border never exceeds
/// `columns`. `title_prefix` is the precomputed SGR prefix applied only to
/// the inline title text so the border itself stays dim/plain.
pub(super) fn build_horizontal_border(args: BorderArgs<'_>) -> String {
    let BorderArgs {
        title,
        left_corner,
        right_corner,
        columns,
        left_pad,
        title_prefix,
    } = args;
    let pad = " ".repeat(left_pad);
    let outer = usize::from(columns);
    let inner = outer.saturating_sub(left_pad.saturating_mul(2));
    // READ-6 / TASK-1973: measured with the ANSI-aware helper, like every
    // other width in this module — `title` can carry an SGR sequence from a
    // report producer or a themed `plan_header_prefix`.
    let corner_l_w = visible_width(left_corner);
    let corner_r_w = visible_width(right_corner);
    // CL-3 / TASK-1969: the corners are non-negotiable, so the title only
    // owns whatever interior is left once both are paid for. A title wider
    // than that (a long `Running: ...` step list, a report producer's own
    // heading) previously pushed the closing corner past `columns` and
    // wrapped the border onto a second line. Clamp with the module's
    // ANSI-aware truncator -- the same helper every other width in this file
    // uses -- and measure the *clamped* text for the fill, so the rendered
    // border is at most `columns` wide.
    let interior = inner.saturating_sub(corner_l_w.saturating_add(corner_r_w));
    let title = truncate_to_width(title, interior);
    let title_w = visible_width(&title);
    let fill = interior.saturating_sub(title_w);
    let fill_str = "─".repeat(fill);
    let colored_title = apply_with_prefix(&title, title_prefix);
    format!("{pad}{left_corner}{colored_title}{fill_str}{right_corner}")
}
