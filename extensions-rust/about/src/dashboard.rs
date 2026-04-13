//! Dashboard command: comprehensive project health page.

use std::io::{self, IsTerminal};

use ops_cargo_toml::CargoToml;
use ops_core::style::{cyan, dim, green, red};
use ops_extension::Context;
use ops_tools::{get_active_toolchain, ToolInfo, ToolStatus};

use crate::format::{
    coverage_icon, format_crates_section, format_dependencies_section, format_description,
    format_header, format_workspace_info,
};
use crate::query::{
    maybe_spinner, query_coverage_data, query_deps_data, query_deps_tree_data,
    query_language_stats, query_loc_data, resolve_member_globs, CoverageData, DepsData,
    DepsTreeData, LanguageStat, LocData,
};
use crate::text_util::{format_number, tty_style};

/// Options for the dashboard command.
pub struct DashboardOptions {
    /// Skip test coverage collection.
    pub skip_coverage: bool,
    /// Force re-collection of data (ignores cached results).
    pub refresh: bool,
}

/// Run the dashboard command, displaying comprehensive project health.
pub fn run_dashboard(
    data_registry: &ops_extension::DataRegistry,
    opts: &DashboardOptions,
    tools: &[ToolInfo],
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(ops_core::config::Config::default());
    let mut ctx = Context::new(config, cwd.clone());
    if opts.refresh {
        ctx.refresh = true;
    }

    let value = ctx.get_or_provide("cargo_toml", data_registry)?;
    let mut manifest: CargoToml = serde_json::from_value((*value).clone())?;

    if let Some(ws) = &mut manifest.workspace {
        ws.members = resolve_member_globs(&ws.members, &cwd);
    }

    let loc_data = query_loc_data(&manifest, &mut ctx, data_registry);
    let deps_data = query_deps_data(&mut ctx, data_registry);
    let deps_tree = query_deps_tree_data(&mut ctx, data_registry);
    let language_stats = query_language_stats(&mut ctx, data_registry);

    let coverage_data = if !opts.skip_coverage {
        let spinner = maybe_spinner("Collecting coverage data\u{2026}");
        let result = query_coverage_data(&manifest, &cwd, &mut ctx, data_registry);
        if let Some(sp) = spinner {
            sp.finish_and_clear();
        }
        result
    } else {
        None
    };

    let toolchain = get_active_toolchain();

    let output = format_dashboard(&DashboardContext {
        manifest: &manifest,
        cwd: &cwd,
        loc_data: loc_data.as_ref(),
        deps_data: deps_data.as_ref(),
        deps_tree: deps_tree.as_ref(),
        coverage_data: coverage_data.as_ref(),
        language_stats: language_stats.as_deref(),
        toolchain: toolchain.as_deref(),
        tools,
    });

    println!("{}", output);

    Ok(())
}

struct DashboardContext<'a> {
    manifest: &'a CargoToml,
    cwd: &'a std::path::Path,
    loc_data: Option<&'a LocData>,
    deps_data: Option<&'a DepsData>,
    deps_tree: Option<&'a DepsTreeData>,
    coverage_data: Option<&'a CoverageData>,
    language_stats: Option<&'a [LanguageStat]>,
    toolchain: Option<&'a str>,
    tools: &'a [ToolInfo],
}

fn format_dashboard(ctx: &DashboardContext<'_>) -> String {
    let project_loc = ctx.loc_data.map(|d| d.project_total);
    let project_file_count = ctx.loc_data.map(|d| d.project_file_count);
    let crate_locs = ctx
        .loc_data
        .filter(|d| !d.per_crate.is_empty())
        .map(|d| &d.per_crate);
    let crate_file_counts = ctx
        .loc_data
        .filter(|d| !d.per_crate_files.is_empty())
        .map(|d| &d.per_crate_files);
    let crate_deps = ctx
        .deps_data
        .filter(|d| !d.per_crate.is_empty())
        .map(|d| &d.per_crate);

    let mut lines = Vec::new();

    // 1. Project Identity
    lines.extend(format_header(&ctx.manifest.package));
    lines.extend(format_description(&ctx.manifest.package));

    // 2. Workspace Overview (with coverage line if available)
    lines.extend(format_workspace_info(
        ctx.manifest,
        ctx.cwd,
        project_loc,
        project_file_count,
        ctx.coverage_data,
    ));

    // 3. Crate Cards
    lines.extend(format_crates_section(
        ctx.manifest,
        ctx.cwd,
        crate_locs,
        crate_file_counts,
        crate_deps,
    ));

    // 4. Code Statistics
    lines.extend(format_language_stats_section(ctx.language_stats));

    // 5. Dependencies
    lines.extend(format_dependencies_section(ctx.manifest, ctx.deps_tree));

    // 6. Test Coverage (detailed table, only if coverage_data has per-crate data)
    lines.extend(format_dashboard_coverage_section(
        ctx.manifest,
        ctx.coverage_data,
        ctx.cwd,
    ));

    // 8. Toolchain & Tools
    lines.extend(format_toolchain_section(ctx.toolchain, ctx.tools));

    lines.join("\n")
}

fn format_language_stats_section(stats: Option<&[LanguageStat]>) -> Vec<String> {
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

fn format_dashboard_coverage_section(
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

    let is_tty = io::stdout().is_terminal();
    lines.push(format!(
        "    {} overall: {:.1}% lines ({} / {})",
        coverage_icon(cov_data.project.lines_percent),
        cov_data.project.lines_percent,
        tty_style(
            &format_number(cov_data.project.lines_covered),
            green,
            is_tty
        ),
        format_number(cov_data.project.lines_count),
    ));
    lines.push(String::new());

    lines.extend(
        crate::format::format_coverage_table(ws, cov_data, workspace_root)
            .lines()
            .map(|l| format!("    {l}")),
    );

    lines
}

fn format_toolchain_section(toolchain: Option<&str>, tools: &[ToolInfo]) -> Vec<String> {
    if toolchain.is_none() && tools.is_empty() {
        return vec![];
    }

    let is_tty = io::stdout().is_terminal();
    let mut lines = vec![String::new(), "  TOOLCHAIN & TOOLS".to_string()];

    if let Some(tc) = toolchain {
        lines.push(String::new());
        lines.push(format!(
            "  \u{25b8} toolchain   {}",
            tty_style(tc, dim, is_tty)
        ));
    }

    if !tools.is_empty() {
        lines.push(String::new());

        let max_name_len = tools.iter().map(|t| t.name.len()).max().unwrap_or(0);

        for tool in tools {
            let (icon, status_text) = match tool.status {
                ToolStatus::Installed => (green("\u{2713}"), ""),
                ToolStatus::NotInstalled => (red("\u{2717}"), " (NOT INSTALLED)"),
                ToolStatus::Unknown => (dim("?"), " (UNKNOWN)"),
            };

            let padded_name = format!("{:width$}", tool.name, width = max_name_len);
            lines.push(format!(
                "    {} {}  {}{}",
                icon,
                tty_style(&padded_name, cyan, is_tty),
                tty_style(&tool.description, dim, is_tty),
                tty_style(status_text, dim, is_tty),
            ));
        }
    }

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
    fn format_toolchain_section_empty() {
        assert!(format_toolchain_section(None, &[]).is_empty());
    }

    #[test]
    fn format_toolchain_section_with_toolchain() {
        let result = format_toolchain_section(Some("stable-aarch64-apple-darwin"), &[]);
        let output = result.join("\n");
        assert!(output.contains("TOOLCHAIN"), "got: {output}");
        assert!(
            output.contains("stable-aarch64-apple-darwin"),
            "got: {output}"
        );
    }

    #[test]
    fn format_toolchain_section_with_tools() {
        let tools = vec![
            ToolInfo {
                name: "cargo-fmt".to_string(),
                description: "Format code".to_string(),
                status: ToolStatus::Installed,
                has_rustup_component: false,
            },
            ToolInfo {
                name: "cargo-nextest".to_string(),
                description: "Better test runner".to_string(),
                status: ToolStatus::NotInstalled,
                has_rustup_component: false,
            },
        ];
        let result = format_toolchain_section(Some("stable"), &tools);
        let output = result.join("\n");
        assert!(output.contains("cargo-fmt"), "got: {output}");
        assert!(output.contains("cargo-nextest"), "got: {output}");
        assert!(output.contains("NOT INSTALLED"), "got: {output}");
    }

    #[test]
    fn format_dashboard_coverage_section_none() {
        let manifest = test_manifest();
        assert!(
            format_dashboard_coverage_section(&manifest, None, std::path::Path::new("/tmp"))
                .is_empty()
        );
    }

    #[test]
    fn format_dashboard_coverage_section_with_data() {
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
        let result =
            format_dashboard_coverage_section(&manifest, Some(&cov), std::path::Path::new("/tmp"));
        let output = result.join("\n");
        assert!(output.contains("TEST COVERAGE"), "got: {output}");
        assert!(output.contains("85.0%"), "got: {output}");
    }
}
