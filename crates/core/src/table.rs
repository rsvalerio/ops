//! Centralized table rendering via `OpsTable` wrapper around comfy_table.

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
    pub fn new() -> Self {
        Self::with_tty(crate::style::stdout_is_terminal())
    }

    /// Create a new table with explicit TTY control (useful for tests).
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
    pub fn is_tty(&self) -> bool {
        self.is_tty
    }

    /// Create a cell that is colored only when outputting to a TTY.
    pub fn cell(&self, value: &str, color: Color) -> Cell {
        if self.is_tty {
            Cell::new(value).fg(color)
        } else {
            Cell::new(value)
        }
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
        assert!(output.contains("a"));
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

    /// PERF-3 / TASK-1439: repeated `OpsTable::new` calls must not re-invoke
    /// `stdout().is_terminal()`. We assert the probe counter advances by at
    /// most one across N constructions: zero when the OnceLock was already
    /// primed by another test in the same process, one when this test
    /// happens to be the first to call it.
    #[test]
    fn new_memoises_is_terminal_probe() {
        let before = crate::style::stdout_is_terminal_probe_count();
        for _ in 0..16 {
            let _ = OpsTable::new();
        }
        let after = crate::style::stdout_is_terminal_probe_count();
        assert!(
            after - before <= 1,
            "stdout is_terminal probed {} times across 16 constructions; expected ≤1",
            after - before
        );
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
