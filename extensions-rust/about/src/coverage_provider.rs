//! Rust `project_coverage` data provider.

use ops_core::project_identity::{CoverageStats, ProjectCoverage, UnitCoverage};
use ops_duckdb::sql::{query_crate_coverage, query_project_coverage};
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::load_workspace_manifest;
use crate::units::resolve_crate_display_name;

pub(crate) const PROVIDER_NAME: &str = "project_coverage";

pub(crate) struct RustCoverageProvider;

impl DataProvider for RustCoverageProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();
        let manifest = load_workspace_manifest(ctx).ok();

        let Some(db) = ops_duckdb::get_db(ctx) else {
            return Ok(serde_json::to_value(ProjectCoverage::default())?);
        };

        // ERR-2 / TASK-0376: surface query failures at warn so a DuckDB
        // schema mismatch / migration bug does not silently render as
        // "0% coverage".
        let total = match query_project_coverage(db) {
            Ok(p) => CoverageStats {
                lines_percent: p.lines_percent,
                lines_covered: p.lines_covered,
                lines_count: p.lines_count,
            },
            Err(e) => {
                tracing::warn!(
                    query = "query_project_coverage",
                    "duckdb query failed; reporting empty coverage: {e:#}"
                );
                return Ok(serde_json::to_value(ProjectCoverage::default())?);
            }
        };

        let units = if let Some(manifest) = manifest {
            let members = manifest
                .workspace
                .as_ref()
                .map(|ws| ws.members.clone())
                .unwrap_or_default();
            if members.is_empty() {
                Vec::new()
            } else {
                let workspace_root = cwd.to_string_lossy();
                let member_strs: Vec<&str> = members.iter().map(String::as_str).collect();
                let per_crate = match query_crate_coverage(db, &member_strs, &workspace_root) {
                    Ok(map) => map,
                    Err(e) => {
                        tracing::warn!(
                            query = "query_crate_coverage",
                            "duckdb query failed; per-crate coverage will be blank: {e:#}"
                        );
                        Default::default()
                    }
                };
                members
                    .iter()
                    .filter_map(|member| {
                        per_crate.get(member).map(|cov| UnitCoverage {
                            unit_name: resolve_crate_display_name(member, &cwd),
                            unit_path: member.clone(),
                            stats: CoverageStats {
                                lines_percent: cov.lines_percent,
                                lines_covered: cov.lines_covered,
                                lines_count: cov.lines_count,
                            },
                        })
                    })
                    .collect()
            }
        } else {
            Vec::new()
        };

        let coverage = ProjectCoverage { total, units };
        serde_json::to_value(&coverage).map_err(DataProviderError::from)
    }
}
