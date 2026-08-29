//! Report rendering.
//!
//! ARCH-1 / TASK-1981: split out of `configurable.rs`. A [`Report`] is
//! rendered through the same `render_slot` seam the runner's step line uses,
//! so "command output" commands match the runner's themed look; this module
//! holds only the report-specific mapping (icons and colours from the
//! `[report]` block, the title/footer chrome, and the boxed variant).

use ops_core::config::theme_types::LayoutKind;
use ops_core::report::{Report, ReportStatus};

use super::boxed::{build_horizontal_border, BorderArgs};
use super::ConfigurableTheme;
use crate::render::sanitise;
use crate::step_line_theme::SlotLine;
use crate::style::apply_with_prefix;

impl ConfigurableTheme {
    /// Icon glyph for a [`ReportStatus`], from the theme's `[report]` block.
    #[must_use]
    pub fn report_icon(&self, status: ReportStatus) -> &str {
        self.config.report.icon(status)
    }

    /// Precomputed SGR prefix for a [`ReportStatus`] result slot.
    fn report_prefix(&self, status: ReportStatus) -> Option<&str> {
        match status {
            ReportStatus::Ok => self.report_ok_prefix.as_deref(),
            ReportStatus::Info => self.report_info_prefix.as_deref(),
            ReportStatus::Warning => self.report_warning_prefix.as_deref(),
            ReportStatus::Error => self.report_error_prefix.as_deref(),
            // `ReportStatus` is `#[non_exhaustive]`; an unknown future severity
            // renders its result slot uncolored rather than failing to build.
            _ => None,
        }
    }

    /// Render a [`Report`] (title, status rows, footer) through the shared
    /// step-line machinery so "command output" commands match the runner's
    /// themed look. Each row becomes a [`SlotLine`] whose icon/color come from
    /// the `[report]` block and whose trailing slot is the result string;
    /// per-row `details` are emitted verbatim (no dotted separator).
    ///
    /// Honors the active layout: flat themes render a bare title + rows +
    /// summary; boxed themes draw the same enclosing frame the runner uses,
    /// with the report title in the top border and the footer in the bottom.
    #[must_use]
    pub fn render_report(&self, report: &Report, columns: u16) -> Vec<String> {
        if matches!(self.config.layout_kind, LayoutKind::Boxed) {
            return self.render_report_boxed(report, columns);
        }

        let pad = self.left_pad_str();
        let mut out = Vec::with_capacity(report.rows.len().saturating_mul(2).saturating_add(4));
        out.push(String::new());
        let title = apply_with_prefix(&report.title, self.report_title_prefix.as_deref());
        out.push(format!("{pad}{title}"));
        out.push(String::new());
        for row in &report.rows {
            out.push(self.render_slot(&self.report_slot(row), columns));
            for detail in &row.details {
                // SEC-21 / TASK-1965: report details are producer-supplied
                // text (often captured tool output) rendered verbatim.
                // CL-3: carry the same left margin the title and rows use —
                // an unpadded detail line hangs one column left of the row it
                // belongs to under any theme with `left_pad > 0`.
                out.push(format!("{pad}{}", sanitise(detail)));
            }
        }
        // Blank line before the summary, matching the runner's flat layout
        // (`✓ … 5.97s` ⏎ blank ⏎ ` Done …`).
        out.push(String::new());
        out.push(self.render_summary_text(&report.footer_text()));
        out
    }

    /// Boxed-layout report: top border with the title, each row wrapped in the
    /// `│ … │` frame (reserving the box columns exactly as the runner's
    /// `render_and_wrap_step` does — render at the reduced budget, then wrap),
    /// detail lines wrapped as continuation content, and a bottom border
    /// carrying the footer summary.
    fn render_report_boxed(&self, report: &Report, columns: u16) -> Vec<String> {
        let mut out = Vec::with_capacity(report.rows.len().saturating_mul(2).saturating_add(4));
        let reserve = self.step_column_reserve();
        let effective = columns.saturating_sub(reserve);

        out.push(String::new());
        out.push(build_horizontal_border(BorderArgs {
            title: &format!(" {} ", report.title),
            left_corner: "╭─",
            right_corner: "╮",
            columns,
            left_pad: self.left_pad(),
            title_prefix: self.report_title_prefix.as_deref(),
        }));
        for row in &report.rows {
            let inner = self.render_slot(&self.report_slot(row), effective);
            // All report rows are terminal; use the runner's "done" cell so the
            // left progress column reads as a solid bar.
            out.push(self.wrap_step_line(&inner, "█", columns));
            for detail in &row.details {
                // SEC-21 / TASK-1965: same untrusted-input class as the flat
                // path above.
                out.push(self.wrap_box_content(&sanitise(detail), columns));
            }
        }
        out.push(build_horizontal_border(BorderArgs {
            title: &format!(" {} ", report.footer_text()),
            left_corner: "╰─",
            right_corner: "╯",
            columns,
            left_pad: self.left_pad(),
            title_prefix: self.summary_prefix.as_deref(),
        }));
        out
    }

    /// Build the [`SlotLine`] for a report row (icon/color from the `[report]`
    /// block, trailing = the result string). Shared by the flat and boxed paths.
    fn report_slot<'a>(&'a self, row: &'a ops_core::report::ReportRow) -> SlotLine<'a> {
        SlotLine {
            icon: self.report_icon(row.status),
            label: &row.label,
            trailing: &row.result,
            trailing_prefix: self.report_prefix(row.status),
            is_running: false,
        }
    }
}
