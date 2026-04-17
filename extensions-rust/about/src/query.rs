//! Data querying: fetch LOC, deps, and coverage from DuckDB/providers.

use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::Path;

use ops_duckdb::sql::{
    query_crate_coverage, query_crate_dep_counts, query_crate_deps, query_crate_file_count,
    query_crate_loc, query_project_coverage, query_project_file_count, query_project_loc,
    CrateCoverage,
};
use ops_extension::Context;

use ops_cargo_toml::CargoToml;

/// LOC data for display — always has project total, workspaces also get per-crate.
#[allow(dead_code)]
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

/// Extract the DuckDb handle from context.
fn get_db(ctx: &Context) -> Option<&ops_duckdb::DuckDb> {
    ops_duckdb::get_db(ctx)
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

/// Bootstrap the required data providers, logging on failure. Returns `None` on any error.
fn bootstrap_providers(
    ctx: &mut Context,
    data_registry: &ops_extension::DataRegistry,
    providers: &[(&str, &str)],
) -> Option<()> {
    for (name, label) in providers {
        if let Err(e) = ctx.get_or_provide(name, data_registry) {
            tracing::debug!("{label}: {e:#}");
            return None;
        }
    }
    Some(())
}

/// Run a per-crate query, logging and returning an empty map on failure.
fn try_per_crate<T, F>(
    label: &str,
    strs: Option<&[&str]>,
    f: F,
) -> std::collections::HashMap<String, T>
where
    F: FnOnce(&[&str]) -> anyhow::Result<std::collections::HashMap<String, T>>,
{
    strs.and_then(|s| match f(s) {
        Ok(v) => Some(v),
        Err(e) => {
            tracing::debug!("{label}: {e:#}");
            None
        }
    })
    .unwrap_or_default()
}

pub(crate) fn query_loc_data(
    manifest: &CargoToml,
    ctx: &mut Context,
    data_registry: &ops_extension::DataRegistry,
) -> Option<LocData> {
    bootstrap_providers(
        ctx,
        data_registry,
        &[
            ("duckdb", "loc: duckdb provider failed"),
            ("tokei", "loc: tokei provider failed"),
        ],
    )?;

    let db = get_db(ctx)?;

    let project_total = query_project_loc(db)
        .map_err(|e| {
            tracing::debug!("loc: query_project_loc failed: {e:#}");
        })
        .ok()?;

    let project_file_count = query_project_file_count(db).unwrap_or_else(|e| {
        tracing::debug!("loc: query_project_file_count failed: {e:#}");
        0
    });

    let member_strs: Option<Vec<&str>> = manifest
        .workspace
        .as_ref()
        .filter(|ws| !ws.members.is_empty())
        .map(|ws| ws.members.iter().map(|s| s.as_str()).collect());

    let per_crate = try_per_crate("loc: query_crate_loc failed", member_strs.as_deref(), |s| {
        query_crate_loc(db, s)
    });
    let per_crate_files = try_per_crate(
        "loc: query_crate_file_count failed",
        member_strs.as_deref(),
        |s| query_crate_file_count(db, s),
    );

    Some(LocData {
        project_total,
        per_crate,
        project_file_count,
        per_crate_files,
    })
}

pub(crate) fn query_deps_data(
    ctx: &mut Context,
    data_registry: &ops_extension::DataRegistry,
) -> Option<DepsData> {
    ctx.get_or_provide("metadata", data_registry).ok()?;
    let db = get_db(ctx)?;
    let per_crate = query_crate_dep_counts(db).ok()?;
    Some(DepsData { per_crate })
}

pub(crate) fn query_deps_tree_data(
    ctx: &mut Context,
    data_registry: &ops_extension::DataRegistry,
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
    data_registry: &ops_extension::DataRegistry,
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

pub(crate) fn query_language_stats(
    ctx: &mut Context,
    data_registry: &ops_extension::DataRegistry,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_member_globs_expands_glob() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/bar")).unwrap();
        std::fs::write(
            root.join("crates/bar/Cargo.toml"),
            "[package]\nname=\"bar\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/not-a-crate")).unwrap();

        let members = vec!["crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);

        assert_eq!(resolved.len(), 2);
        assert!(resolved.contains(&"crates/bar".to_string()));
        assert!(resolved.contains(&"crates/foo".to_string()));
        assert_eq!(resolved[0], "crates/bar");
        assert_eq!(resolved[1], "crates/foo");
    }

    #[test]
    fn resolve_member_globs_non_glob_passthrough() {
        let members = vec!["crates/core".to_string(), "crates/cli".to_string()];
        let resolved = resolve_member_globs(&members, std::path::Path::new("/nonexistent"));
        assert_eq!(
            resolved,
            vec!["crates/cli".to_string(), "crates/core".to_string()]
        );
    }

    #[test]
    fn resolve_member_globs_mixed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();

        let members = vec!["explicit".to_string(), "crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);

        assert_eq!(resolved.len(), 2);
        assert!(resolved.contains(&"explicit".to_string()));
        assert!(resolved.contains(&"crates/foo".to_string()));
    }

    #[test]
    fn resolve_member_globs_no_matching_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        // Create the parent dir but no children with Cargo.toml
        std::fs::create_dir_all(root.join("crates")).unwrap();

        let members = vec!["crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_member_globs_nonexistent_glob_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        // Don't create the "crates" directory at all
        let members = vec!["crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_member_globs_empty_members() {
        let resolved = resolve_member_globs(&[], std::path::Path::new("/nonexistent"));
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_member_globs_sorted_output() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        // Create members in reverse alphabetical order
        for name in &["zebra", "alpha", "middle"] {
            std::fs::create_dir_all(root.join(format!("crates/{name}"))).unwrap();
            std::fs::write(
                root.join(format!("crates/{name}/Cargo.toml")),
                format!("[package]\nname=\"{name}\"\n"),
            )
            .unwrap();
        }

        let members = vec!["crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);

        assert_eq!(resolved.len(), 3);
        assert_eq!(resolved[0], "crates/alpha");
        assert_eq!(resolved[1], "crates/middle");
        assert_eq!(resolved[2], "crates/zebra");
    }

    #[test]
    fn resolve_member_globs_ignores_files_not_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates")).unwrap();
        // Create a file (not a directory) in crates/
        std::fs::write(root.join("crates/not-a-dir"), "some content").unwrap();
        // Create a valid crate dir
        std::fs::create_dir_all(root.join("crates/real")).unwrap();
        std::fs::write(
            root.join("crates/real/Cargo.toml"),
            "[package]\nname=\"real\"\n",
        )
        .unwrap();

        let members = vec!["crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);

        assert_eq!(resolved, vec!["crates/real"]);
    }

    #[test]
    fn resolve_member_globs_deep_glob_prefix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("extensions/rust/foo")).unwrap();
        std::fs::write(
            root.join("extensions/rust/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();

        let members = vec!["extensions/rust/*".to_string()];
        let resolved = resolve_member_globs(&members, root);

        assert_eq!(resolved, vec!["extensions/rust/foo"]);
    }

    #[test]
    fn maybe_spinner_returns_none_in_non_tty() {
        // In test environments, stderr is not a TTY
        let result = maybe_spinner("test message");
        assert!(result.is_none());
    }
}
