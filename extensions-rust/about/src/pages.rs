//! About subpages: detailed views for coverage, code, dependencies, and crates.

use std::io::{self, IsTerminal};

use ops_cargo_toml::CargoToml;
use ops_core::style::green;
use ops_extension::Context;

use crate::format::{
    coverage_icon, format_crates_section, format_dependencies_section, format_description,
    format_header,
};
use crate::query::{
    maybe_spinner, query_coverage_data, query_deps_data, query_deps_tree_data,
    query_language_stats, query_loc_data, resolve_member_globs, CoverageData, LanguageStat,
};
use crate::text_util::{format_number, tty_style};

/// A single about subpage to render in isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AboutPage {
    Coverage,
    Code,
    Dependencies,
    Crates,
}

/// Render a single about subpage with the Rust stack's detailed view.
///
/// Each page queries only the data it needs, then renders the corresponding
/// section.
pub fn run_about_page(
    data_registry: &ops_extension::DataRegistry,
    page: AboutPage,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = Context::new(config, cwd.clone());

    let value = ctx.get_or_provide("cargo_toml", data_registry)?;
    let mut manifest: CargoToml = serde_json::from_value((*value).clone())?;

    if let Some(ws) = &mut manifest.workspace {
        ws.members = resolve_member_globs(&ws.members, &cwd);
    }

    let mut lines = Vec::new();

    let needs_header = !matches!(page, AboutPage::Coverage);
    if needs_header {
        lines.extend(format_header(&manifest.package));
        lines.extend(format_description(&manifest.package));
    }

    match page {
        AboutPage::Coverage => {
            let coverage_data = {
                let spinner = maybe_spinner("Collecting coverage data\u{2026}");
                let result = query_coverage_data(&manifest, &cwd, &mut ctx, data_registry);
                if let Some(sp) = spinner {
                    sp.finish_and_clear();
                }
                result
            };
            lines.extend(format_coverage_section(
                &manifest,
                coverage_data.as_ref(),
                &cwd,
            ));
        }
        AboutPage::Code => {
            let language_stats = query_language_stats(&mut ctx, data_registry);
            lines.extend(format_language_stats_section(language_stats.as_deref()));
        }
        AboutPage::Dependencies => {
            let _ = ctx.get_or_provide("duckdb", data_registry);
            let deps_tree = query_deps_tree_data(&mut ctx, data_registry);
            lines.extend(format_dependencies_section(&manifest, deps_tree.as_ref()));
        }
        AboutPage::Crates => {
            let loc_data = query_loc_data(&manifest, &mut ctx, data_registry);
            let deps_data = query_deps_data(&mut ctx, data_registry);
            let crate_locs = loc_data
                .as_ref()
                .filter(|d| !d.per_crate.is_empty())
                .map(|d| &d.per_crate);
            let crate_file_counts = loc_data
                .as_ref()
                .filter(|d| !d.per_crate_files.is_empty())
                .map(|d| &d.per_crate_files);
            let crate_deps = deps_data
                .as_ref()
                .filter(|d| !d.per_crate.is_empty())
                .map(|d| &d.per_crate);
            lines.extend(format_crates_section(
                &manifest,
                &cwd,
                crate_locs,
                crate_file_counts,
                crate_deps,
            ));
        }
    }

    println!("{}", lines.join("\n"));
    Ok(())
}

pub(crate) fn format_language_stats_section(stats: Option<&[LanguageStat]>) -> Vec<String> {
    let stats = match stats {
        Some(s) if !s.is_empty() => s,
        _ => return vec![],
    };

    use ops_core::table::{Cell, OpsTable};

    let mut table = OpsTable::new();
    table.set_header(vec!["Language", "Lines of Code", "Files"]);

    let total_loc: i64 = stats.iter().map(|s| s.loc).sum();

    for stat in stats {
        let pct = if total_loc > 0 {
            format!("{:.1}%", (stat.loc as f64 / total_loc as f64) * 100.0)
        } else {
            String::new()
        };
        let loc_str = format!("{} ({})", format_number(stat.loc), pct);
        table.add_row(vec![
            Cell::new(&stat.language),
            Cell::new(&loc_str),
            Cell::new(format_number(stat.file_count)),
        ]);
    }

    let mut lines = vec![
        String::new(),
        "  CODE STATISTICS".to_string(),
        String::new(),
    ];
    lines.extend(table.to_string().lines().map(|l| format!("    {l}")));
    lines
}

pub(crate) fn format_coverage_section(
    manifest: &CargoToml,
    coverage_data: Option<&CoverageData>,
    workspace_root: &std::path::Path,
) -> Vec<String> {
    let cov_data = match coverage_data {
        Some(d) if d.project.lines_count > 0 => d,
        _ => return vec![],
    };

    let ws = match &manifest.workspace {
        Some(ws) if !ws.members.is_empty() && !cov_data.per_crate.is_empty() => ws,
        _ => return vec![],
    };

    let mut lines = vec![String::new(), "  TEST COVERAGE".to_string(), String::new()];

    lines.extend(
        crate::format::format_coverage_table(ws, cov_data, workspace_root)
            .lines()
            .map(|l| format!("    {l}")),
    );

    let is_tty = io::stdout().is_terminal();
    lines.push(String::new());
    lines.push(format!(
        "    {} total: {:.1}% lines ({} / {})",
        coverage_icon(cov_data.project.lines_percent),
        cov_data.project.lines_percent,
        tty_style(
            &format_number(cov_data.project.lines_covered),
            green,
            is_tty
        ),
        format_number(cov_data.project.lines_count),
    ));

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn test_manifest() -> CargoToml {
        let toml_str = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[workspace]
members = ["crates/core", "crates/cli"]
"#;
        toml::from_str(toml_str).expect("test manifest should parse")
    }

    #[test]
    fn format_language_stats_section_empty() {
        assert!(format_language_stats_section(None).is_empty());
        assert!(format_language_stats_section(Some(&[])).is_empty());
    }

    #[test]
    fn format_language_stats_section_with_data() {
        let stats = vec![
            LanguageStat {
                language: "Rust".to_string(),
                loc: 8000,
                file_count: 40,
            },
            LanguageStat {
                language: "TOML".to_string(),
                loc: 200,
                file_count: 5,
            },
        ];
        let result = format_language_stats_section(Some(&stats));
        let output = result.join("\n");
        assert!(output.contains("CODE STATISTICS"), "got: {output}");
        assert!(output.contains("Rust"), "got: {output}");
        assert!(output.contains("TOML"), "got: {output}");
    }

    #[test]
    fn format_coverage_section_none() {
        let manifest = test_manifest();
        assert!(format_coverage_section(&manifest, None, std::path::Path::new("/tmp")).is_empty());
    }

    #[test]
    fn format_coverage_section_with_data() {
        let manifest = test_manifest();
        let mut per_crate = HashMap::new();
        per_crate.insert(
            "crates/core".to_string(),
            ops_duckdb::sql::CrateCoverage {
                lines_percent: 85.0,
                lines_covered: 850,
                lines_count: 1000,
            },
        );
        let cov = CoverageData {
            project: ops_duckdb::sql::CrateCoverage {
                lines_percent: 85.0,
                lines_covered: 850,
                lines_count: 1000,
            },
            per_crate,
        };
        let result = format_coverage_section(&manifest, Some(&cov), std::path::Path::new("/tmp"));
        let output = result.join("\n");
        assert!(output.contains("TEST COVERAGE"), "got: {output}");
        assert!(output.contains("85.0%"), "got: {output}");
        assert!(output.contains("total:"), "got: {output}");
    }

    #[test]
    fn format_coverage_section_zero_lines_count_returns_empty() {
        let manifest = test_manifest();
        let cov = CoverageData {
            project: ops_duckdb::sql::CrateCoverage {
                lines_percent: 0.0,
                lines_covered: 0,
                lines_count: 0,
            },
            per_crate: HashMap::new(),
        };
        assert!(
            format_coverage_section(&manifest, Some(&cov), std::path::Path::new("/tmp")).is_empty()
        );
    }

    #[test]
    fn format_coverage_section_empty_workspace_members_returns_empty() {
        let toml_str = r#"
[package]
name = "test-project"
version = "0.1.0"

[workspace]
members = []
"#;
        let manifest: CargoToml = toml::from_str(toml_str).unwrap();
        let mut per_crate = HashMap::new();
        per_crate.insert(
            "crates/core".to_string(),
            ops_duckdb::sql::CrateCoverage {
                lines_percent: 50.0,
                lines_covered: 50,
                lines_count: 100,
            },
        );
        let cov = CoverageData {
            project: ops_duckdb::sql::CrateCoverage {
                lines_percent: 50.0,
                lines_covered: 50,
                lines_count: 100,
            },
            per_crate,
        };
        assert!(
            format_coverage_section(&manifest, Some(&cov), std::path::Path::new("/tmp")).is_empty()
        );
    }

    #[test]
    fn format_coverage_section_no_workspace_returns_empty() {
        let toml_str = r#"
[package]
name = "single-crate"
version = "0.1.0"
"#;
        let manifest: CargoToml = toml::from_str(toml_str).unwrap();
        let cov = CoverageData {
            project: ops_duckdb::sql::CrateCoverage {
                lines_percent: 80.0,
                lines_covered: 80,
                lines_count: 100,
            },
            per_crate: HashMap::new(),
        };
        assert!(
            format_coverage_section(&manifest, Some(&cov), std::path::Path::new("/tmp")).is_empty()
        );
    }

    #[test]
    fn format_coverage_section_empty_per_crate_returns_empty() {
        let manifest = test_manifest();
        let cov = CoverageData {
            project: ops_duckdb::sql::CrateCoverage {
                lines_percent: 80.0,
                lines_covered: 80,
                lines_count: 100,
            },
            per_crate: HashMap::new(),
        };
        assert!(
            format_coverage_section(&manifest, Some(&cov), std::path::Path::new("/tmp")).is_empty()
        );
    }

    #[test]
    fn format_language_stats_section_single_language_shows_100_percent() {
        let stats = vec![LanguageStat {
            language: "Rust".to_string(),
            loc: 5000,
            file_count: 25,
        }];
        let result = format_language_stats_section(Some(&stats));
        let output = result.join("\n");
        assert!(output.contains("100.0%"), "got: {output}");
        assert!(output.contains("Rust"), "got: {output}");
    }

    #[test]
    fn format_language_stats_section_percentages_add_up() {
        let stats = vec![
            LanguageStat {
                language: "Rust".to_string(),
                loc: 750,
                file_count: 10,
            },
            LanguageStat {
                language: "TOML".to_string(),
                loc: 250,
                file_count: 5,
            },
        ];
        let result = format_language_stats_section(Some(&stats));
        let output = result.join("\n");
        assert!(output.contains("75.0%"), "got: {output}");
        assert!(output.contains("25.0%"), "got: {output}");
    }

    #[test]
    fn about_page_enum_equality() {
        assert_eq!(AboutPage::Coverage, AboutPage::Coverage);
        assert_ne!(AboutPage::Coverage, AboutPage::Code);
        assert_ne!(AboutPage::Dependencies, AboutPage::Crates);
    }

    #[test]
    fn about_page_enum_debug() {
        assert_eq!(format!("{:?}", AboutPage::Coverage), "Coverage");
        assert_eq!(format!("{:?}", AboutPage::Code), "Code");
        assert_eq!(format!("{:?}", AboutPage::Dependencies), "Dependencies");
        assert_eq!(format!("{:?}", AboutPage::Crates), "Crates");
    }

    #[test]
    fn about_page_enum_clone() {
        let page = AboutPage::Crates;
        let cloned = page;
        assert_eq!(page, cloned);
    }
}
