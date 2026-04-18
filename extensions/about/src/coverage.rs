//! Stack-agnostic `about coverage` subpage: per-unit coverage table + totals.
//!
//! Calls the `project_coverage` data provider registered by the active stack.

use std::io::IsTerminal;

use ops_core::project_identity::ProjectCoverage;
use ops_core::style::green;
use ops_core::table::{Color, OpsTable};
use ops_core::text::format_number;
use ops_extension::{Context, DataProviderError, DataRegistry};

use crate::text_util::tty_style;

pub const PROJECT_COVERAGE_PROVIDER: &str = "project_coverage";

enum CoverageTier {
    Low,
    Medium,
    High,
}

fn coverage_tier(pct: f64) -> CoverageTier {
    if pct < 50.0 {
        CoverageTier::Low
    } else if pct < 80.0 {
        CoverageTier::Medium
    } else {
        CoverageTier::High
    }
}

/// Status icon for coverage percentage.
pub fn coverage_icon(pct: f64) -> &'static str {
    match coverage_tier(pct) {
        CoverageTier::Low => "\u{1f480}",           // skull
        CoverageTier::Medium => "\u{26a0}\u{fe0f}", // warning
        CoverageTier::High => "\u{2705}",           // check mark
    }
}

/// Table cell color for coverage percentage.
pub fn coverage_color(pct: f64) -> Color {
    match coverage_tier(pct) {
        CoverageTier::Low => Color::Red,
        CoverageTier::Medium => Color::Yellow,
        CoverageTier::High => Color::Green,
    }
}

pub fn run_about_coverage(data_registry: &DataRegistry) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = Context::new(config, cwd);

    // Warm the providers stacks may rely on for coverage + unit metadata.
    let _ = ctx.get_or_provide("duckdb", data_registry);
    let _ = ctx.get_or_provide("coverage", data_registry);
    let _ = ctx.get_or_provide("cargo_toml", data_registry);

    let coverage = match ctx.get_or_provide(PROJECT_COVERAGE_PROVIDER, data_registry) {
        Ok(v) => serde_json::from_value::<ProjectCoverage>((*v).clone())?,
        Err(DataProviderError::NotFound(_)) => ProjectCoverage::default(),
        Err(e) => return Err(e.into()),
    };

    if coverage.total.lines_count == 0 {
        println!("No coverage data available.");
        return Ok(());
    }

    let lines = format_coverage_section(&coverage);
    println!("{}", lines.join("\n"));
    Ok(())
}

pub fn format_coverage_section(coverage: &ProjectCoverage) -> Vec<String> {
    if coverage.total.lines_count == 0 {
        return vec![];
    }

    let is_tty = std::io::stdout().is_terminal();
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

    lines
}

fn format_coverage_table(units: &[&ops_core::project_identity::UnitCoverage]) -> String {
    let mut table = OpsTable::new();
    table.set_header(vec!["", "Unit", "Coverage", "Covered", "Total"]);

    let mut sorted: Vec<&&ops_core::project_identity::UnitCoverage> = units.iter().collect();
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
    fn format_coverage_section_empty_when_zero_total() {
        let cov = ProjectCoverage::default();
        assert!(format_coverage_section(&cov).is_empty());
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
        let out = format_coverage_section(&cov).join("\n");
        assert!(out.contains("85.0%"));
        assert!(out.contains("total:"));
        assert!(out.contains("core"));
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
        let out = format_coverage_section(&cov).join("\n");
        assert!(out.contains("active"));
        assert!(!out.contains("empty"));
    }
}
