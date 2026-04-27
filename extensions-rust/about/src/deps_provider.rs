//! Rust `project_dependencies` data provider.
//!
//! Queries DuckDB for per-crate direct dependencies via cargo metadata.

use ops_core::project_identity::{ProjectDependencies, UnitDeps};
use ops_duckdb::sql::query_crate_deps;
use ops_extension::{Context, DataProvider, DataProviderError};

pub(crate) const PROVIDER_NAME: &str = "project_dependencies";

pub(crate) struct RustDepsProvider;

impl DataProvider for RustDepsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let Some(db) = ops_duckdb::get_db(ctx) else {
            return Ok(serde_json::to_value(ProjectDependencies::default())?);
        };

        // ERR-2 / TASK-0376: a DuckDB schema/migration error here used to
        // surface as an empty deps list with no signal. Log at warn before
        // falling back so the failure is visible.
        let per_crate = match query_crate_deps(db) {
            Ok(map) => map,
            Err(e) => {
                tracing::warn!(
                    query = "query_crate_deps",
                    "duckdb query failed; project_dependencies will be empty: {e:#}"
                );
                Default::default()
            }
        };
        let units: Vec<UnitDeps> = per_crate
            .into_iter()
            .map(|(unit_name, deps)| UnitDeps { unit_name, deps })
            .collect();

        let result = ProjectDependencies { units };
        serde_json::to_value(&result).map_err(DataProviderError::from)
    }
}
