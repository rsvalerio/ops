//! Stack-agnostic `about coverage` subpage: per-unit coverage table + totals.
//!
//! Calls the `project_coverage` data provider registered by the active stack.

use std::io::{IsTerminal, Write};

use ops_core::project_identity::ProjectCoverage;
use ops_core::style::green;
use ops_core::table::{Color, OpsTable};
use ops_core::text::format_number;
use ops_extension::{Context, DataRegistry};

use crate::providers::{load_or_default, warm_providers};
use crate::text_util::tty_style;

pub const PROJECT_COVERAGE_PROVIDER: &str = "project_coverage";

enum CoverageTier {
    Low,
    Medium,
    High,
}

fn coverage_tier(pct: f64) -> CoverageTier {
    if pct.is_nan() || pct < 50.0 {
        CoverageTier::Low
    } else if pct < 80.0 {
        CoverageTier::Medium
    } else {
        CoverageTier::High
    }
}

/// Status icon for coverage percentage.
pub(crate) fn coverage_icon(pct: f64) -> &'static str {
    match coverage_tier(pct) {
        CoverageTier::Low => "\u{1f480}",           // skull
        CoverageTier::Medium => "\u{26a0}\u{fe0f}", // warning
        CoverageTier::High => "\u{2705}",           // check mark
    }
}

/// Table cell color for coverage percentage.
pub(crate) fn coverage_color(pct: f64) -> Color {
    match coverage_tier(pct) {
        CoverageTier::Low => Color::Red,
        CoverageTier::Medium => Color::Yellow,
        CoverageTier::High => Color::Green,
    }
}

pub fn run_about_coverage(data_registry: &DataRegistry) -> anyhow::Result<()> {
    let is_tty = std::io::stdout().is_terminal();
    run_about_coverage_with(data_registry, &mut std::io::stdout(), is_tty)
}

/// READ-5/TASK-0411: `is_tty` reflects the `writer` the caller hands in.
/// See [`crate::units::run_about_units_with`] for the rationale.
pub fn run_about_coverage_with(
    data_registry: &DataRegistry,
    writer: &mut dyn Write,
    is_tty: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = Context::new(config, cwd);

    // Warm the providers stacks may rely on for coverage + unit metadata.
    warm_providers(
        &mut ctx,
        data_registry,
        &["duckdb", "coverage", "cargo_toml"],
        "coverage",
    );

    let coverage: ProjectCoverage =
        load_or_default(&mut ctx, data_registry, PROJECT_COVERAGE_PROVIDER)?;

    match format_coverage_section(&coverage, is_tty) {
        Some(lines) => writeln!(writer, "{}", lines.join("\n"))?,
        None => writeln!(writer, "No coverage data available.")?,
    }
    Ok(())
}

/// Format coverage data as a displayable section.
///
/// Returns `None` when `lines_count == 0` (no coverage data collected),
/// signalling the caller to emit a user-friendly message. All other cases
/// return `Some(lines)`.
pub fn format_coverage_section(coverage: &ProjectCoverage, is_tty: bool) -> Option<Vec<String>> {
    if coverage.total.lines_count == 0 {
        return None;
    }

    let mut lines = vec![String::new()];

    let active_units: Vec<&_> = coverage
        .units
        .iter()
        .filter(|u| u.stats.lines_count > 0)
        .collect();

    if !active_units.is_empty() {
        lines.extend(
            format_coverage_table(&active_units)
                .lines()
                .map(|l| format!("    {l}")),
        );
        lines.push(String::new());
    }

    lines.push(format!(
        "    {} total: {:.1}% lines ({} / {})",
        coverage_icon(coverage.total.lines_percent),
        coverage.total.lines_percent,
        tty_style(&format_number(coverage.total.lines_covered), green, is_tty),
        format_number(coverage.total.lines_count),
    ));

    Some(lines)
}

fn format_coverage_table(units: &[&ops_core::project_identity::UnitCoverage]) -> String {
    let mut table = OpsTable::new();
    table.set_header(vec!["", "Unit", "Coverage", "Covered", "Total"]);

    let mut sorted: Vec<&ops_core::project_identity::UnitCoverage> = units.to_vec();
    sorted.sort_by(|a, b| a.unit_path.cmp(&b.unit_path));

    for u in &sorted {
        let pct = u.stats.lines_percent;
        let color = coverage_color(pct);
        let icon = coverage_icon(pct);
        let pct_str = format!("{:.1}%", pct);
        table.add_row(vec![
            table.cell(icon, color),
            table.cell(&u.unit_name, color),
            table.cell(&pct_str, color),
            table.cell(&format_number(u.stats.lines_covered), color),
            table.cell(&format_number(u.stats.lines_count), color),
        ]);
    }

    table.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::{CoverageStats, UnitCoverage};

    #[test]
    fn coverage_icon_thresholds() {
        assert_eq!(coverage_icon(0.0), "\u{1f480}");
        assert_eq!(coverage_icon(50.0), "\u{26a0}\u{fe0f}");
        assert_eq!(coverage_icon(80.0), "\u{2705}");
    }

    #[test]
    fn coverage_color_thresholds() {
        assert!(matches!(coverage_color(0.0), Color::Red));
        assert!(matches!(coverage_color(50.0), Color::Yellow));
        assert!(matches!(coverage_color(80.0), Color::Green));
    }

    #[test]
    fn coverage_tier_classifies_nan_as_low() {
        assert!(matches!(coverage_tier(f64::NAN), CoverageTier::Low));
    }

    #[test]
    fn coverage_icon_nan_is_skull_not_checkmark() {
        assert_eq!(coverage_icon(f64::NAN), "\u{1f480}");
    }

    #[test]
    fn coverage_color_nan_is_red_not_green() {
        assert!(matches!(coverage_color(f64::NAN), Color::Red));
    }

    #[test]
    fn format_coverage_section_none_when_zero_total() {
        let cov = ProjectCoverage::default();
        assert!(format_coverage_section(&cov, false).is_none());
    }

    #[test]
    fn format_coverage_section_with_units() {
        let cov = ProjectCoverage {
            total: CoverageStats {
                lines_percent: 85.0,
                lines_covered: 850,
                lines_count: 1000,
            },
            units: vec![UnitCoverage {
                unit_name: "core".to_string(),
                unit_path: "crates/core".to_string(),
                stats: CoverageStats {
                    lines_percent: 85.0,
                    lines_covered: 850,
                    lines_count: 1000,
                },
            }],
        };
        let lines = format_coverage_section(&cov, false).expect("non-zero coverage returns Some");
        // Structural contract, not substring: one blank, N table lines, one blank, total.
        assert!(lines.len() >= 4, "got lines: {lines:?}");
        assert!(lines.first().unwrap().is_empty(), "leading blank");
        // The last line is the project total in a fixed format.
        let total = lines.last().unwrap();
        assert_eq!(
            total,
            &format!(
                "    {} total: 85.0% lines (850 / 1,000)",
                coverage_icon(85.0)
            ),
            "total line format changed: {total}"
        );
        // The blank separator sits immediately before the total.
        assert!(
            lines[lines.len() - 2].is_empty(),
            "blank before total: {:?}",
            lines
        );
        // Exactly one table block between the leading blank and the pre-total blank.
        let table_block: Vec<&String> = lines[1..lines.len() - 2].iter().collect();
        assert!(
            table_block.iter().all(|l| l.starts_with("    ")),
            "every table line is indented with 4 spaces: {table_block:?}"
        );
        assert!(
            table_block.iter().any(|l| l.contains("core")),
            "unit name present in table: {table_block:?}"
        );
    }

    #[test]
    fn format_coverage_section_skips_zero_unit() {
        let cov = ProjectCoverage {
            total: CoverageStats {
                lines_percent: 80.0,
                lines_covered: 80,
                lines_count: 100,
            },
            units: vec![
                UnitCoverage {
                    unit_name: "active".to_string(),
                    unit_path: "crates/active".to_string(),
                    stats: CoverageStats {
                        lines_percent: 80.0,
                        lines_covered: 80,
                        lines_count: 100,
                    },
                },
                UnitCoverage {
                    unit_name: "empty".to_string(),
                    unit_path: "crates/empty".to_string(),
                    stats: CoverageStats::default(),
                },
            ],
        };
        let lines = format_coverage_section(&cov, false).expect("non-zero coverage returns Some");
        // Same structural contract as the units test: active unit appears
        // exactly once in the table block, empty unit is filtered out, and
        // the total line matches the pinned format.
        let total = lines.last().expect("has total line");
        assert_eq!(
            total,
            &format!("    {} total: 80.0% lines (80 / 100)", coverage_icon(80.0))
        );
        let table_block = &lines[1..lines.len() - 2];
        let active_hits = table_block.iter().filter(|l| l.contains("active")).count();
        assert_eq!(
            active_hits, 1,
            "active appears once in table: {table_block:?}"
        );
        assert!(
            !table_block.iter().any(|l| l.contains("empty")),
            "empty unit filtered out: {table_block:?}"
        );
    }
}
