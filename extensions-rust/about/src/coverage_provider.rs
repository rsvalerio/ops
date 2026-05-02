//! Rust `project_coverage` data provider.

use ops_core::project_identity::{CoverageStats, ProjectCoverage, UnitCoverage};
use ops_duckdb::sql::{query_crate_coverage, query_or_warn, query_project_coverage};
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::{load_workspace_manifest, log_manifest_load_failure};
use crate::units::resolve_crate_display_name;

pub(crate) const PROVIDER_NAME: &str = "project_coverage";

pub(crate) struct RustCoverageProvider;

impl DataProvider for RustCoverageProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();
        let manifest = match load_workspace_manifest(ctx) {
            Ok(m) => Some(m),
            Err(e) => {
                log_manifest_load_failure(&e);
                None
            }
        };

        let Some(db) = ops_duckdb::get_db(ctx) else {
            return Ok(serde_json::to_value(ProjectCoverage::default())?);
        };

        // ERR-2 / TASK-0376 / PATTERN-1 (TASK-0608): route through
        // `query_or_warn` so this site matches the convention used by every
        // sister DuckDB call in the crate (units, identity::metrics,
        // deps_provider). Wrapping the return in `Option` preserves the
        // early-return-on-failure semantics — if the project_coverage query
        // fails we return a fully-default `ProjectCoverage` rather than
        // partial data, matching the prior behaviour.
        let project_total = query_or_warn(
            "query_project_coverage",
            "reporting empty coverage",
            None,
            || query_project_coverage(db).map(Some),
        );
        let Some(p) = project_total else {
            return Ok(serde_json::to_value(ProjectCoverage::default())?);
        };
        let total = CoverageStats::new(p.lines_percent, p.lines_covered, p.lines_count);

        let units = if let Some(manifest) = manifest {
            let members: &[String] = manifest
                .workspace
                .as_ref()
                .map_or(&[][..], |ws| ws.members.as_slice());
            if members.is_empty() {
                Vec::new()
            } else {
                // PERF-3 / TASK-0917: borrow the cwd inline. `to_string_lossy`
                // returns a `Cow::Borrowed` for UTF-8 paths (no allocation),
                // and the temporary lives for the duration of the closure
                // body — no need for an explicit let-bind on the hot path.
                let member_strs: Vec<&str> = members.iter().map(String::as_str).collect();
                let per_crate = query_or_warn(
                    "query_crate_coverage",
                    "per-crate coverage will be blank",
                    std::collections::HashMap::<String, ops_duckdb::sql::CrateCoverage>::new(),
                    || query_crate_coverage(db, &member_strs, &cwd.to_string_lossy()),
                );
                // PERF-1 (TASK-0798): resolve display names up front in one
                // pass over members with coverage rows, so each member's
                // Cargo.toml is read at most once per provide() call.
                let mut display_names: std::collections::HashMap<&str, String> =
                    std::collections::HashMap::with_capacity(per_crate.len());
                for member in members {
                    if per_crate.contains_key(member.as_str()) {
                        display_names
                            .insert(member.as_str(), resolve_crate_display_name(member, &cwd));
                    }
                }
                members
                    .iter()
                    .filter_map(|member| {
                        let cov = per_crate.get(member)?;
                        let unit_name = display_names.remove(member.as_str())?;
                        Some(UnitCoverage::new(
                            unit_name,
                            member.clone(),
                            CoverageStats::new(
                                cov.lines_percent,
                                cov.lines_covered,
                                cov.lines_count,
                            ),
                        ))
                    })
                    .collect()
            }
        } else {
            Vec::new()
        };

        let coverage = ProjectCoverage::new(total, units);
        serde_json::to_value(&coverage).map_err(DataProviderError::from)
    }
}
