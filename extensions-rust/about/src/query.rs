//! Data querying: fetch LOC, deps, coverage, and updates from DuckDB/providers.

use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::Path;

use cargo_ops_duckdb::sql::{
    query_crate_coverage, query_crate_dep_counts, query_crate_deps, query_crate_file_count,
    query_crate_loc, query_project_coverage, query_project_file_count, query_project_loc,
    CrateCoverage,
};
use cargo_ops_extension::Context;

use cargo_ops_cargo_toml::CargoToml;

/// LOC data for display — always has project total, workspaces also get per-crate.
pub(crate) struct LocData {
    /// Total LOC for the whole project (shown in workspace info).
    pub(crate) project_total: i64,
    /// Per-crate LOC for workspace members (shown on cards). Empty for single-crate.
    pub(crate) per_crate: HashMap<String, i64>,
    /// Total file count for the whole project (shown in workspace info).
    pub(crate) project_file_count: i64,
    /// Per-crate file counts for workspace members (shown on cards). Empty for single-crate.
    pub(crate) per_crate_files: HashMap<String, i64>,
}

/// Dependency count data for display on crate cards.
pub(crate) struct DepsData {
    /// Per-crate dep counts: package_name -> normal dep count.
    pub(crate) per_crate: HashMap<String, i64>,
}

/// Per-crate dependency tree data for the DEPENDENCIES section.
pub(crate) struct DepsTreeData {
    /// crate_name -> Vec<(dep_name, version_req)>
    pub(crate) per_crate: HashMap<String, Vec<(String, String)>>,
}

/// Coverage data for display — project total + per-crate breakdown.
pub(crate) struct CoverageData {
    /// Total coverage for the whole project (shown in workspace info).
    pub(crate) project: CrateCoverage,
    /// Per-crate coverage for workspace members (shown in COVERAGE section).
    pub(crate) per_crate: HashMap<String, CrateCoverage>,
}

/// Parsed updates data for the UPDATES section.
pub(crate) struct UpdatesData {
    pub(crate) result: cargo_ops_cargo_update::CargoUpdateResult,
}

/// Per-language LOC breakdown for the CODE STATISTICS section.
pub(crate) struct LanguageStat {
    pub(crate) language: String,
    pub(crate) loc: i64,
    pub(crate) file_count: i64,
}

/// Create a spinner on stderr if the terminal is interactive.
pub(crate) fn maybe_spinner(message: &str) -> Option<indicatif::ProgressBar> {
    if !std::io::stderr().is_terminal() {
        return None;
    }
    let sp = indicatif::ProgressBar::new_spinner();
    sp.set_style(
        indicatif::ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .expect("static spinner template is valid")
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
    );
    sp.set_message(message.to_string());
    sp.enable_steady_tick(std::time::Duration::from_millis(80));
    Some(sp)
}

/// Extract the DuckDb handle from context (DUP-007).
fn get_db(ctx: &Context) -> Option<&cargo_ops_duckdb::DuckDb> {
    ctx.db
        .as_ref()
        .and_then(|h| h.as_any().downcast_ref::<cargo_ops_duckdb::DuckDb>())
}

/// Expand workspace member glob patterns (e.g. `crates/*`) into actual directory paths
/// relative to the workspace root. Non-glob entries are passed through as-is.
pub(crate) fn resolve_member_globs(members: &[String], workspace_root: &Path) -> Vec<String> {
    let mut resolved = Vec::new();
    for member in members {
        if member.contains('*') {
            if let Some(idx) = member.find('*') {
                let prefix = &member[..idx];
                let parent = workspace_root.join(prefix);
                if let Ok(entries) = std::fs::read_dir(&parent) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("Cargo.toml").exists() {
                            if let Ok(rel) = path.strip_prefix(workspace_root) {
                                resolved.push(rel.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        } else {
            resolved.push(member.clone());
        }
    }
    resolved.sort();
    resolved
}

pub(crate) fn query_loc_data(
    manifest: &CargoToml,
    ctx: &mut Context,
    data_registry: &cargo_ops_extension::DataRegistry,
) -> Option<LocData> {
    if let Err(e) = ctx.get_or_provide("duckdb", data_registry) {
        tracing::debug!("loc: duckdb provider failed: {e:#}");
        return None;
    }
    if let Err(e) = ctx.get_or_provide("tokei", data_registry) {
        tracing::debug!("loc: tokei provider failed: {e:#}");
        return None;
    }

    let db = get_db(ctx)?;

    let project_total = match query_project_loc(db) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("loc: query_project_loc failed: {e:#}");
            return None;
        }
    };

    let project_file_count = match query_project_file_count(db) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("loc: query_project_file_count failed: {e:#}");
            0
        }
    };

    let member_strs: Option<Vec<&str>> = manifest
        .workspace
        .as_ref()
        .filter(|ws| !ws.members.is_empty())
        .map(|ws| ws.members.iter().map(|s| s.as_str()).collect());

    let per_crate = member_strs
        .as_deref()
        .and_then(|strs| match query_crate_loc(db, strs) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::debug!("loc: query_crate_loc failed: {e:#}");
                None
            }
        })
        .unwrap_or_default();

    let per_crate_files = member_strs
        .as_deref()
        .and_then(|strs| match query_crate_file_count(db, strs) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::debug!("loc: query_crate_file_count failed: {e:#}");
                None
            }
        })
        .unwrap_or_default();

    Some(LocData {
        project_total,
        per_crate,
        project_file_count,
        per_crate_files,
    })
}

pub(crate) fn query_deps_data(
    ctx: &mut Context,
    data_registry: &cargo_ops_extension::DataRegistry,
) -> Option<DepsData> {
    ctx.get_or_provide("metadata", data_registry).ok()?;
    let db = get_db(ctx)?;
    let per_crate = query_crate_dep_counts(db).ok()?;
    Some(DepsData { per_crate })
}

pub(crate) fn query_deps_tree_data(
    ctx: &mut Context,
    data_registry: &cargo_ops_extension::DataRegistry,
) -> Option<DepsTreeData> {
    ctx.get_or_provide("metadata", data_registry).ok()?;
    let db = get_db(ctx)?;
    let per_crate = query_crate_deps(db).ok()?;
    Some(DepsTreeData { per_crate })
}

pub(crate) fn query_coverage_data(
    manifest: &CargoToml,
    cwd: &std::path::Path,
    ctx: &mut Context,
    data_registry: &cargo_ops_extension::DataRegistry,
) -> Option<CoverageData> {
    if let Err(e) = ctx.get_or_provide("duckdb", data_registry) {
        tracing::debug!("coverage: duckdb provider failed: {e:#}");
        return None;
    }
    if let Err(e) = ctx.get_or_provide("coverage", data_registry) {
        tracing::debug!("coverage: coverage provider failed: {e:#}");
        return None;
    }

    let db = get_db(ctx)?;

    let project = match query_project_coverage(db) {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!("coverage: query_project_coverage failed: {e:#}");
            return None;
        }
    };

    let workspace_root = cwd.to_string_lossy();
    let per_crate = manifest
        .workspace
        .as_ref()
        .filter(|ws| !ws.members.is_empty())
        .and_then(|ws| {
            let member_strs: Vec<&str> = ws.members.iter().map(|s| s.as_str()).collect();
            match query_crate_coverage(db, &member_strs, &workspace_root) {
                Ok(v) => Some(v),
                Err(e) => {
                    tracing::debug!("coverage: query_crate_coverage failed: {e:#}");
                    None
                }
            }
        })
        .unwrap_or_default();

    Some(CoverageData { project, per_crate })
}

pub(crate) fn query_updates_data(
    ctx: &mut Context,
    data_registry: &cargo_ops_extension::DataRegistry,
) -> Option<UpdatesData> {
    let value = ctx.get_or_provide("cargo_update", data_registry).ok()?;
    let result: cargo_ops_cargo_update::CargoUpdateResult =
        serde_json::from_value((*value).clone()).ok()?;
    Some(UpdatesData { result })
}

pub(crate) fn query_language_stats(
    ctx: &mut Context,
    data_registry: &cargo_ops_extension::DataRegistry,
) -> Option<Vec<LanguageStat>> {
    if let Err(e) = ctx.get_or_provide("duckdb", data_registry) {
        tracing::debug!("language_stats: duckdb provider failed: {e:#}");
        return None;
    }
    if let Err(e) = ctx.get_or_provide("tokei", data_registry) {
        tracing::debug!("language_stats: tokei provider failed: {e:#}");
        return None;
    }

    let db = get_db(ctx)?;
    let conn = db.lock().ok()?;

    let mut stmt = match conn.prepare(
        "SELECT language, SUM(code) as loc, COUNT(*) as file_count \
         FROM tokei_files GROUP BY language ORDER BY loc DESC",
    ) {
        Ok(s) => s,
        Err(e) => {
            tracing::debug!("language_stats: prepare failed (table may not exist): {e:#}");
            return None;
        }
    };

    let rows = stmt
        .query_map([], |row| {
            Ok(LanguageStat {
                language: row.get(0)?,
                loc: row.get(1)?,
                file_count: row.get(2)?,
            })
        })
        .ok()?;

    let stats: Vec<LanguageStat> = rows.filter_map(|r| r.ok()).collect();
    if stats.is_empty() {
        None
    } else {
        Some(stats)
    }
}
