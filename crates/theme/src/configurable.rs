//! TOML-configurable theme implementation.
//!
//! # Width measurement
//!
//! READ-6 / TASK-1973: **every** string measured in this module's
//! render/layout pipeline is measured with [`visible_width`], the ANSI-aware
//! helper. The module previously mixed it with `ops_core::output::display_width`
//! (a bare `UnicodeWidthStr::width` with no ANSI grammar), so a label, title
//! or icon carrying an escape sequence got two different widths depending on
//! which half of a line was being computed, and the two halves disagreed by
//! exactly the escape's byte length. Those values are not hypothetical:
//! report labels and results come from tool output, and the icon glyphs and
//! `plan_header_prefix` come from user TOML. Consistency here is what keeps
//! the boxed frame straight, and it is the precondition for the truncation
//! policy below.
//!
//! # Truncation policy
//!
//! CL-3 / TASK-1969: no rendered line may exceed the column budget it was
//! given. The budget is spent in this order — chrome (indent, icon column,
//! the space after it), the trailing slot, [`MIN_SEP_GLYPHS`] separator
//! columns — and the **label** absorbs whatever is left, truncated through
//! [`truncate_to_width`] (which marks the cut with an ellipsis). Each frame
//! wrapper (`wrap_step_line`, `wrap_box_content`, the error-block right pad)
//! independently clamps its content to its own interior budget, so the
//! closing bar always lands in the same column as the borders. A budget of
//! `0` columns means "unknown"; nothing is clamped in that case.

use ops_core::output::{StepLine, StepStatus, ALL_STATUSES};

use super::step_line_theme::{format_duration, SlotLine, StepPrefixParts};
use super::style::{apply_with_prefix, precompute_sgr_prefix, truncate_to_width, visible_width};
use super::{PlanHeaderStyle, ThemeConfig};

mod boxed;
mod config_access;
mod report;

/// Minimum separator run, in terminal columns, between the label and the
/// trailing slot.
const MIN_SEP_GLYPHS: usize = 3;

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
    /// Precomputed SGR prefixes for the report `[report]` block — one per
    /// [`ReportStatus`] result slot, plus the report title. Mirrors the
    /// label/separator/duration prefixes so `render_report` avoids re-parsing
    /// the color specs on every row.
    report_ok_prefix: Option<String>,
    report_info_prefix: Option<String>,
    report_warning_prefix: Option<String>,
    report_error_prefix: Option<String>,
    report_title_prefix: Option<String>,
    /// TASK-1035: precomputed `" ".repeat(config.left_pad)` so the per-step
    /// render path doesn't allocate a fresh padding string on every call.
    left_pad_str: String,
    /// PERF-3 / TASK-1975: the widest [`ALL_STATUSES`] icon, in columns.
    /// Derived only from `config`, which is moved in here and never mutated,
    /// so it is constant for the theme's lifetime — but it was measured
    /// afresh on every rendered row (and again per error block) before being
    /// hoisted into the same precomputed block as the SGR prefixes.
    icon_column_width: usize,
    /// READ-5 / TASK-1971: columns the spinner cell occupies on a running
    /// row — the widest glyph in `config.tick_chars`, not a literal `1`.
    /// An emoji tick set is two columns wide and would otherwise push the
    /// running row's icon column one cell right of every completed row.
    spinner_reserve_cols: usize,
    /// READ-5 / TASK-1971: display columns of `config.separator_char`. The
    /// separator budget is computed in columns, so a full-width separator
    /// glyph must yield half as many repetitions rather than a line twice
    /// as wide as its budget.
    separator_char_cols: usize,
}

impl ConfigurableTheme {
    #[must_use]
    pub fn new(config: ThemeConfig) -> Self {
        let left_pad_str = " ".repeat(config.left_pad);
        let icon_column_width = ALL_STATUSES
            .iter()
            .map(|s| visible_width(config.status_icon(*s)))
            .max()
            .unwrap_or(0);
        // A zero-width tick or separator glyph would make the column budget
        // meaningless (and divide by zero below), so both floor at one cell.
        let spinner_reserve_cols = config
            .tick_chars
            .chars()
            .map(|c| visible_width(c.encode_utf8(&mut [0u8; 4])))
            .max()
            .unwrap_or(1)
            .max(1);
        let separator_char_cols =
            visible_width(config.separator_char.encode_utf8(&mut [0u8; 4])).max(1);
        warn_on_running_template_overhead(&config, spinner_reserve_cols);
        Self {
            header_prefix: precompute_sgr_prefix(&config.header_color),
            summary_prefix: precompute_sgr_prefix(&config.summary_color),
            label_prefix: precompute_sgr_prefix(&config.label_color),
            separator_prefix: precompute_sgr_prefix(&config.separator_color),
            duration_prefix: precompute_sgr_prefix(&config.duration_color),
            report_ok_prefix: precompute_sgr_prefix(&config.report.color_ok),
            report_info_prefix: precompute_sgr_prefix(&config.report.color_info),
            report_warning_prefix: precompute_sgr_prefix(&config.report.color_warning),
            report_error_prefix: precompute_sgr_prefix(&config.report.color_error),
            report_title_prefix: precompute_sgr_prefix(&config.report.title_color),
            left_pad_str,
            icon_column_width,
            spinner_reserve_cols,
            separator_char_cols,
            config,
        }
    }

    /// Width of the icon column: the widest [`ALL_STATUSES`] glyph.
    ///
    /// PERF-3 / TASK-1975: computed once in [`Self::new`] and stored, like
    /// the SGR prefixes and `left_pad_str` beside it. The signature is
    /// unchanged, so callers on the per-row render path are unaffected.
    #[must_use]
    pub const fn icon_column_width(&self) -> usize {
        self.icon_column_width
    }

    #[must_use]
    pub fn render_plan_header(&self, command_ids: &[String]) -> Vec<String> {
        let pad = self.left_pad_str();
        let ids = command_ids.join(", ");
        match self.config.plan_header_style {
            PlanHeaderStyle::Plain => {
                let body = format!("{}Running: {}", self.config.plan_header_prefix, ids);
                let colored = apply_with_prefix(&body, self.header_prefix.as_deref());
                let header = format!("{pad}{colored}");
                vec![String::new(), header, String::new()]
            }
            PlanHeaderStyle::Tree => {
                let body = format!("┌ Running: {ids}");
                let colored = apply_with_prefix(&body, self.header_prefix.as_deref());
                vec![
                    String::new(),
                    format!("{}{}", pad, colored),
                    format!("{}│", pad),
                ]
            }
        }
    }

    #[must_use]
    pub fn render_summary_separator(&self, _columns: u16) -> String {
        if self.config.summary_separator.is_empty() {
            String::new()
        } else {
            format!("{}{}", self.left_pad_str(), self.config.summary_separator)
        }
    }

    /// DUP-5 / TASK-0354: shared layout for the left portion of a step line.
    /// Both [`render`](Self::render) and [`render_prefix`](Self::render_prefix)
    /// need exactly the same indent / icon / padding triple, and the two
    /// outputs must remain byte-identical in their prefix bytes —
    /// `render_separator` derives layout math from `display_width(plain_prefix)`.
    /// Returning the components separately (rather than re-deriving them in
    /// each caller) makes drift impossible.
    #[must_use]
    pub fn step_prefix_parts(&self, status: StepStatus, is_running: bool) -> StepPrefixParts<'_> {
        self.icon_prefix_parts(self.status_icon(status), is_running)
    }

    /// Generalization of [`step_prefix_parts`](Self::step_prefix_parts) over an
    /// explicit icon glyph, so report rows (whose icons come from the `[report]`
    /// block, not [`StepStatus`]) share the exact same icon-column alignment.
    /// `icon_column_width` still measures over [`ALL_STATUSES`], so report
    /// glyphs (✓ ⚠ ✘ ℹ, width 1) line up under the same column as step icons.
    #[must_use]
    pub fn icon_prefix_parts<'a>(&'a self, icon: &'a str, is_running: bool) -> StepPrefixParts<'a> {
        let icon_width = visible_width(icon);
        let max_icon_width = self.icon_column_width();
        // READ-5 / TASK-1971: the running row's spinner cell is as wide as
        // the widest configured tick glyph, not a hardcoded single column.
        let (indent, spinner_cols) = if is_running {
            ("", self.spinner_reserve_cols)
        } else {
            (self.step_indent(), 0usize)
        };
        let pad =
            " ".repeat(max_icon_width.saturating_sub(icon_width.saturating_add(spinner_cols)));
        StepPrefixParts { indent, icon, pad }
    }

    /// Build the left portion of a step line: indent + icon + padding + label.
    #[must_use]
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
    #[must_use]
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
        let reserved_chrome = template_overhead.saturating_add(self.left_pad());
        let line_budget = columns.saturating_sub(reserved_chrome);

        // Fixed costs inside `line_budget`: the label prefix, the duration
        // (when present), and one leading space before the separator.
        let prefix_width = visible_width(prefix);
        let leading_space = 1usize;
        let fixed_inside = prefix_width
            .saturating_add(visible_width(duration_str))
            .saturating_add(leading_space);

        let space_for_sep = line_budget.saturating_sub(fixed_inside);
        // Columns the separator run may occupy, including its leading space.
        let sep_cols = space_for_sep.max(MIN_SEP_GLYPHS);
        let sep = self.separator_char();

        // PERF-3 / TASK-1130: build the leading-space + repeated-sep + optional
        // trailing-space directly into a single String, avoiding the intermediate
        // `sep.to_string().repeat(n)` allocation per step render.
        //
        // READ-5 / TASK-1971: `sep_cols` is a column budget, so the number of
        // glyphs is that budget divided by the glyph's *display width*. A
        // full-width separator (U+FF0E, U+3002, …) configured in `.ops.toml`
        // therefore yields half as many repetitions and the same total width,
        // instead of a line twice as wide as the budget it was derived from.
        let dots_count = sep_cols
            .saturating_sub(1)
            .checked_div(self.separator_char_cols)
            .unwrap_or(0);
        let trailing_space = duration_str.is_empty();
        let sep_len = sep.len_utf8();
        let mut out = String::with_capacity(
            dots_count
                .saturating_mul(sep_len)
                .saturating_add(1)
                .saturating_add(usize::from(trailing_space)),
        );
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
    // the spec string on every step line. The body now lives in
    // [`render_slot`](Self::render_slot); `render` only maps a `StepLine` onto a
    // `SlotLine` (icon = status icon, trailing = formatted duration). Keep this
    // mapping mechanical so the runner output stays byte-identical.
    #[must_use]
    pub fn render(&self, step: &StepLine, columns: u16) -> String {
        let is_running = step.status == StepStatus::Running;
        let trailing = step
            .elapsed
            .map(|d| self.format_elapsed(d))
            .unwrap_or_default();
        self.render_slot(
            &SlotLine {
                icon: self.status_icon(step.status),
                label: &step.label,
                trailing: &trailing,
                trailing_prefix: self.duration_prefix.as_deref(),
                is_running,
            },
            columns,
        )
    }

    /// Shared render path for one line of `{indent}{icon}{pad} label … trailing`,
    /// driven by an explicit [`SlotLine`]. Both the runner's [`render`](Self::render)
    /// (trailing = duration) and report rows (trailing = result string) call
    /// here so the prefix layout, dotted separator, and color application have a
    /// single source of truth.
    #[must_use]
    pub fn render_slot(&self, slot: &SlotLine<'_>, columns: u16) -> String {
        let parts = self.icon_prefix_parts(slot.icon, slot.is_running);
        let budget = usize::from(columns);
        let template_overhead = if slot.is_running {
            self.running_template_overhead()
        } else {
            0
        };
        // CL-3 / TASK-1969: spend the budget on chrome, the trailing slot and
        // the minimum separator run first; the label gets the remainder and is
        // truncated to it. See the module-level truncation policy.
        let label = truncate_to_width(slot.label, self.label_budget(&parts, slot, budget));
        let plain_prefix = format!("{}{}{} {}", parts.indent, parts.icon, parts.pad, label);
        let plain_separator =
            self.render_separator(&plain_prefix, slot.trailing, budget, slot.is_running);
        let pad = if slot.is_running {
            ""
        } else {
            self.left_pad_str()
        };

        let colored_label = apply_with_prefix(&label, self.label_prefix.as_deref());
        let colored_prefix = format!(
            "{}{}{} {}",
            parts.indent, parts.icon, parts.pad, colored_label
        );
        let colored_separator =
            apply_with_prefix(&plain_separator, self.separator_prefix.as_deref());

        let line = if slot.trailing.is_empty() {
            format!("{pad}{colored_prefix}{colored_separator}")
        } else {
            let colored_trailing = apply_with_prefix(slot.trailing, slot.trailing_prefix);
            format!("{pad}{colored_prefix}{colored_separator} {colored_trailing}")
        };
        if budget == 0 {
            return line;
        }
        // Belt and braces: the label budget above already makes the line fit,
        // but a trailing slot wider than the whole budget (a report result
        // string, say) has no label left to give back — clamp regardless so
        // the invariant "a rendered line fits its budget" holds for every
        // input, not only the ones the arithmetic anticipated.
        truncate_to_width(&line, budget.saturating_sub(template_overhead)).into_owned()
    }

    /// Columns available to the label in [`render_slot`], after the chrome,
    /// the trailing slot and the minimum separator run are reserved.
    fn label_budget(
        &self,
        parts: &StepPrefixParts<'_>,
        slot: &SlotLine<'_>,
        budget: usize,
    ) -> usize {
        if budget == 0 {
            // No budget given: measure nothing, clamp nothing. `usize::MAX`
            // keeps the label intact while still routing it through the
            // control-character stripping in `truncate_to_width`.
            return usize::MAX;
        }
        let template_overhead = if slot.is_running {
            self.running_template_overhead()
        } else {
            0
        };
        // The space between the icon column and the label, plus the space
        // that precedes the trailing slot (or the separator's own trailing
        // space when there is none).
        let spaces = 2usize;
        let reserved = template_overhead
            .saturating_add(self.left_pad())
            .saturating_add(visible_width(parts.indent))
            .saturating_add(visible_width(parts.icon))
            .saturating_add(parts.pad.len())
            .saturating_add(visible_width(slot.trailing))
            .saturating_add(spaces)
            .saturating_add(MIN_SEP_GLYPHS);
        budget.saturating_sub(reserved)
    }

    // TASK-0747: render_summary uses precomputed SGR prefix. Split so report
    // footers reuse the same chrome (`render_summary_text`) with their own body.
    #[must_use]
    pub fn render_summary(&self, success: bool, elapsed_secs: f64) -> String {
        let label = if success { "Done" } else { "Failed" };
        self.render_summary_text(&format!("{} in {}", label, format_duration(elapsed_secs)))
    }

    /// Render an arbitrary summary body with the theme's summary chrome
    /// (left pad + summary glyph/separator + colored body). The runner passes
    /// `"Done in 1.20s"`; reports pass their `footer_text()`.
    #[must_use]
    pub fn render_summary_text(&self, body: &str) -> String {
        let colored = apply_with_prefix(body, self.summary_prefix.as_deref());
        format!(
            "{}{}{}",
            self.left_pad_str(),
            self.summary_prefix(),
            colored
        )
    }
}

/// READ-5 / TASK-1971: validate `running_template_overhead` against the
/// template it is supposed to describe, and surface a mismatch instead of
/// silently mis-budgeting every running row.
///
/// The field is a hand-maintained column count that a theme author must keep
/// consistent with `running_template` by eye; nothing derived it and
/// `render_separator` subtracts it from the budget as fact. The literal
/// (non-placeholder) text of the template plus the widest spinner glyph is a
/// *lower bound* on that overhead — the `{elapsed}` placeholder adds more at
/// render time, so a larger configured value is legitimate. A configured
/// value below the bound is not: the separator then over-runs the terminal
/// width on every running row.
fn warn_on_running_template_overhead(config: &ThemeConfig, spinner_cols: usize) {
    let minimum = template_literal_width(&config.running_template).saturating_add(spinner_cols);
    if config.running_template_overhead < minimum {
        ops_core::ui::warn(format!(
            "theme running_template_overhead is {} but the template's literal text and spinner \
             glyph already occupy {minimum} columns; running step lines will over-run the \
             terminal width",
            config.running_template_overhead
        ));
    }
}

/// Display columns of the literal text in an `indicatif` template — that is,
/// everything outside a `{…}` placeholder.
fn template_literal_width(template: &str) -> usize {
    let mut literal = String::with_capacity(template.len());
    let mut depth = 0usize;
    for ch in template.chars() {
        match ch {
            '{' => depth = depth.saturating_add(1),
            '}' => depth = depth.saturating_sub(1),
            c if depth == 0 => literal.push(c),
            _ => {}
        }
    }
    visible_width(&literal)
}
