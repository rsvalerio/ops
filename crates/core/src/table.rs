//! Centralized table rendering via `OpsTable` wrapper around `comfy_table`.

use std::fmt;

use comfy_table::{
    modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, ColumnConstraint, ContentArrangement, Table,
    TableComponent, Width::Fixed,
};

pub use comfy_table::{Cell, Color};

/// A TTY-aware table that centralizes styling and coloring decisions.
#[derive(Debug)]
pub struct OpsTable {
    inner: Table,
    is_tty: bool,
}

impl Default for OpsTable {
    fn default() -> Self {
        Self::new()
    }
}

impl OpsTable {
    /// Create a new table, auto-detecting TTY from stdout.
    ///
    /// PERF-3 / TASK-1439: TTY probe routes through the shared
    /// `style::stdout_is_terminal` cache so repeated constructions reuse
    /// a single `isatty` syscall per process and cannot disagree with
    /// `style::color_enabled` mid-render after a redirect.
    #[must_use]
    pub fn new() -> Self {
        Self::with_tty(crate::style::stdout_is_terminal())
    }

    /// Create a new table with explicit TTY control (useful for tests).
    #[must_use]
    pub fn with_tty(is_tty: bool) -> Self {
        let mut inner = Table::new();
        inner
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_style(TableComponent::HorizontalLines, '─')
            .set_style(TableComponent::HeaderLines, '─')
            .set_style(TableComponent::VerticalLines, '│')
            .set_style(TableComponent::LeftBorderIntersections, '├')
            .set_style(TableComponent::RightBorderIntersections, '┤')
            .set_style(TableComponent::LeftHeaderIntersection, '├')
            .set_style(TableComponent::RightHeaderIntersection, '┤')
            .set_style(TableComponent::MiddleIntersections, '┼')
            .set_style(TableComponent::MiddleHeaderIntersections, '┼');
        Self { inner, is_tty }
    }

    /// Whether this table is rendering for a TTY.
    #[must_use]
    pub const fn is_tty(&self) -> bool {
        self.is_tty
    }

    /// Create a cell that is colored only when outputting to a TTY.
    ///
    /// SEC-11 / TASK-2032: `value` is sanitised by [`sanitise_table_text`],
    /// so this constructor is safe for untrusted text. Colour is applied by
    /// comfy-table via `fg`, never as escape bytes inside `value`, so
    /// stripping controls here cannot swallow a caller's styling.
    #[must_use]
    pub fn cell(&self, value: &str, color: Color) -> Cell {
        let cell = Self::text_cell(value);
        if self.is_tty {
            cell.fg(color)
        } else {
            cell
        }
    }

    /// The documented way to render untrusted text into a table: an
    /// uncoloured cell whose contents have been run through
    /// [`sanitise_table_text`].
    ///
    /// SEC-11 / TASK-2032: `comfy_table::Cell::new` is re-exported from this
    /// module and writes whatever bytes it is handed straight into a cell.
    /// Anything that reaches a table from outside the process — a plan
    /// document, a config value, a subprocess's stdout — must come through
    /// here (or through [`OpsTable::cell`], which delegates to it) instead.
    #[must_use]
    pub fn text_cell(value: &str) -> Cell {
        Cell::new(sanitise_table_text(value))
    }

    /// Set the table header row.
    pub fn set_header(&mut self, headers: Vec<&str>) -> &mut Self {
        self.inner.set_header(headers);
        self
    }

    /// Add a row of cells to the table.
    pub fn add_row(&mut self, cells: Vec<Cell>) -> &mut Self {
        self.inner.add_row(cells);
        self
    }

    /// Set the maximum width for a column.
    pub fn set_max_width(&mut self, column: usize, width: u16) -> &mut Self {
        if let Some(col) = self.inner.column_mut(column) {
            col.set_constraint(ColumnConstraint::UpperBoundary(Fixed(width)));
        }
        self
    }
}

/// SEC-11 / TASK-1939, TASK-2032: strip every character that a terminal or
/// comfy-table would act on rather than draw, so untrusted text can be put
/// in a table cell.
///
/// Table cells are the widest untrusted-input surface the CLI paints:
/// terraform plan documents (`--json-file` accepts an arbitrary path, and
/// even the default path carries names from third-party registry modules),
/// config values, and tool output. A value carrying an ESC-bracket CSI
/// sequence — erase-line plus cursor-up, say — can wipe rows already printed
/// and redraw fabricated ones on exactly the screen an operator reads before
/// approving an apply. A bare carriage return does the same more crudely, and
/// either desynchronises comfy-table's width accounting.
///
/// DUP-2 / TASK-2250: the dropped set *is*
/// [`crate::text::is_unsafe_display_char`] — the whole-codepoint
/// display-safety policy shared with every other operator-facing surface.
/// That covers the `Cc` controls (`U+0000..=U+001F` with ESC, CR, LF and TAB
/// included, plus DEL and the C1 range), the exhaustive `Cf` format category
/// (the bidi overrides / isolates that reverse the rendered order of a name
/// or path, the zero-width family and BOM that forge alignment or split a
/// recognizable token such as `aws_db_instance`, and the script-specific
/// format controls), and the `Zl` / `Zp` line and paragraph separators.
/// Deriving the set from the shared predicate keeps this drop-based consumer
/// in lockstep with the escape-based [`crate::ui::sanitise_line`] channel,
/// which escapes the same set: a codepoint newly added to the policy can no
/// longer reach table cells while every other surface already rejects it.
///
/// The drop-versus-escape decision stays local, as it was before: stripping
/// is right for a fixed-width table cell whose width budget comes from the
/// string it is handed, while the stderr channel escapes offenders into
/// visible `\xNN` / `\u{NNNN}` text so an operator can see they were there.
#[must_use]
pub fn sanitise_table_text(value: &str) -> String {
    value
        .chars()
        .filter(|c| !crate::text::is_unsafe_display_char(*c))
        .collect()
}

impl fmt::Display for OpsTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_tty_false_reports_not_tty() {
        let table = OpsTable::with_tty(false);
        assert!(!table.is_tty());
    }

    #[test]
    fn with_tty_true_reports_tty() {
        let table = OpsTable::with_tty(true);
        assert!(table.is_tty());
    }

    #[test]
    fn cell_renders_content_regardless_of_tty() {
        let mut table = OpsTable::with_tty(false);
        table.set_header(vec!["Val"]);
        let row = vec![table.cell("hello", Color::Cyan)];
        table.add_row(row);
        let output = table.to_string();
        assert!(output.contains("hello"));
    }

    #[test]
    fn set_header_and_add_row() {
        let mut table = OpsTable::with_tty(false);
        table.set_header(vec!["Name", "Value"]);
        let row = vec![table.cell("a", Color::Cyan), table.cell("b", Color::White)];
        table.add_row(row);
        let output = table.to_string();
        assert!(output.contains("Name"));
        assert!(output.contains('a'));
    }

    #[test]
    fn set_max_width_constrains_rendered_column() {
        let mut table = OpsTable::with_tty(false);
        table.set_header(vec!["Col"]);
        // A long cell value that would otherwise expand the column well past 10.
        table.add_row(vec![Cell::new("a".repeat(200))]);
        table.set_max_width(0, 10);
        let rendered = table.to_string();
        // Every rendered line must respect the 10-column upper bound (plus
        // two border chars). Finding a line with 150 `a`s would prove the
        // constraint was not applied.
        assert!(
            !rendered.contains(&"a".repeat(50)),
            "column width constraint not applied: {rendered}"
        );
    }

    #[test]
    fn set_max_width_out_of_range_is_noop() {
        let mut table = OpsTable::with_tty(false);
        table.set_header(vec!["Col"]);
        table.add_row(vec![Cell::new("x")]);
        let before = table.to_string();
        table.set_max_width(99, 20);
        let after = table.to_string();
        assert_eq!(before, after);
    }

    /// PERF-3 / TASK-1439 + TEST-1 / TASK-1856: `OpsTable::new` must obtain
    /// its TTY state from the shared `style::stdout_is_terminal` cache rather
    /// than issuing its own `stdout().is_terminal()`.
    ///
    /// The previous version of this test read a counter incremented *inside*
    /// `OnceLock::get_or_init`, which caps it at 1 per process by
    /// construction — so `after - before <= 1` held no matter what
    /// `OpsTable::new` did, including the pre-TASK-1439 direct `isatty` call
    /// it named as the regression. The counter now advances once per call to
    /// the shared accessor, so a construction that bypasses the cache
    /// contributes nothing and the assertion below fails.
    ///
    /// `>=` rather than `==`: other tests in this binary run in parallel
    /// threads and legitimately consult the same accessor (every
    /// `style::color_enabled` call does), so an exact delta would be flaky.
    /// The direction that matters is the floor — a bypass drops the delta to
    /// zero.
    #[test]
    fn new_routes_tty_probe_through_shared_cache() {
        const CONSTRUCTIONS: usize = 16;
        let before = crate::style::stdout_tty_query_count();
        for _ in 0..CONSTRUCTIONS {
            let _ = OpsTable::new();
        }
        let after = crate::style::stdout_tty_query_count();
        assert!(
            after - before >= CONSTRUCTIONS,
            "shared stdout TTY cache consulted {} times across {CONSTRUCTIONS} constructions; \
             OpsTable::new must route every construction through it",
            after - before
        );
    }

    /// SEC-11 / TASK-2032 AC#4: a control-sequence-bearing value rendered
    /// through the table must leave no byte a terminal acts on. `OpsTable`
    /// is the shared sink for every table in the workspace, so this is the
    /// one place the guarantee has to hold.
    #[test]
    fn rendered_cells_carry_no_control_bytes() {
        let hostile = "aws_db\u{1b}[2J\u{1b}[1A_instance\rforged";
        let mut table = OpsTable::with_tty(false);
        table.set_header(vec!["Name"]);
        table.add_row(vec![OpsTable::text_cell(hostile)]);
        table.add_row(vec![table.cell(hostile, Color::Cyan)]);
        let rendered = table.to_string();
        assert!(!rendered.contains('\u{1b}'), "ESC survived: {rendered}");
        assert!(!rendered.contains('\r'), "CR survived: {rendered}");
        // `\n` is the table's own row separator, so assert per line.
        assert!(
            rendered.lines().all(|l| !l.chars().any(char::is_control)),
            "control byte survived: {rendered}"
        );
        assert!(
            rendered.contains("aws_db[2J[1A_instanceforged"),
            "the text itself must still render: {rendered}"
        );
    }

    /// SEC-11 / TASK-2032: the coloured constructor delegates to the
    /// sanitising one, so a caller cannot pick colour and lose the defence.
    #[test]
    fn cell_sanitises_on_both_tty_paths() {
        for is_tty in [false, true] {
            let table = OpsTable::with_tty(is_tty);
            let mut t = OpsTable::with_tty(false);
            t.add_row(vec![table.cell("a\u{1b}[2Jb", Color::Cyan)]);
            let rendered = t.to_string();
            assert!(!rendered.contains('\u{1b}'), "is_tty={is_tty}: {rendered}");
        }
    }

    /// SEC-11 / TASK-1939 (moved here by TASK-2032): printable text and
    /// non-ASCII scripts survive untouched — this is a control-character
    /// filter, not a charset restriction.
    #[test]
    fn sanitise_table_text_preserves_printable_and_unicode() {
        assert_eq!(
            sanitise_table_text("module.vpc/subnet-\u{f6}\u{e4} \u{540d}\u{524d}"),
            "module.vpc/subnet-\u{f6}\u{e4} \u{540d}\u{524d}"
        );
        assert_eq!(sanitise_table_text("a\tb\nc"), "abc");
        assert_eq!(sanitise_table_text(""), "");
    }

    /// SEC-11 / TASK-1939 (moved here by TASK-2032): the `Cf` format
    /// characters are invisible to width accounting but change what the
    /// operator reads, so they are stripped alongside the `Cc` controls.
    #[test]
    fn sanitise_table_text_strips_bidi_and_zero_width() {
        assert_eq!(
            sanitise_table_text("aws_db_instance.\u{202e}etaerc\u{202c}"),
            "aws_db_instance.etaerc"
        );
        assert_eq!(
            sanitise_table_text("\u{2066}mod\u{2067}ule\u{2068}.a\u{2069}"),
            "module.a"
        );
        assert_eq!(
            sanitise_table_text("aws\u{200b}_db\u{200d}_inst\u{feff}ance"),
            "aws_db_instance"
        );
        assert_eq!(sanitise_table_text("a\u{2060}b\u{200e}c"), "abc");
        for c in ['\u{202a}', '\u{202d}', '\u{2069}', '\u{feff}', '\u{200f}'] {
            assert_eq!(sanitise_table_text(&c.to_string()), "");
        }
    }

    #[test]
    fn display_delegates_to_inner() {
        let mut table = OpsTable::with_tty(false);
        table.set_header(vec!["X"]);
        let row = vec![Cell::new("val")];
        table.add_row(row);
        let displayed = format!("{table}");
        assert!(displayed.contains("val"));
    }
}
