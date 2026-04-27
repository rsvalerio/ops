//! Rust `project_coverage` data provider.

use ops_cargo_toml::CargoToml;
use ops_core::project_identity::{CoverageStats, ProjectCoverage, UnitCoverage};
use ops_duckdb::sql::{query_crate_coverage, query_project_coverage};
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::resolve_member_globs;
use crate::units::resolve_crate_display_name;

pub(crate) const PROVIDER_NAME: &str = "project_coverage";

pub(crate) struct RustCoverageProvider;

impl DataProvider for RustCoverageProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();

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

        let manifest: Option<CargoToml> = ctx
            .cached("cargo_toml")
            .and_then(|v| serde_json::from_value((**v).clone()).ok());

        let units = if let Some(mut manifest) = manifest {
            if let Some(ws) = &mut manifest.workspace {
                ws.members = resolve_member_globs(&ws.members, &cwd);
            }
            match &manifest.workspace {
                Some(ws) if !ws.members.is_empty() => {
                    let workspace_root = cwd.to_string_lossy();
                    let member_strs: Vec<&str> = ws.members.iter().map(|s| s.as_str()).collect();
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
                    ws.members
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
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };

        let coverage = ProjectCoverage { total, units };
        serde_json::to_value(&coverage).map_err(DataProviderError::from)
    }
}
