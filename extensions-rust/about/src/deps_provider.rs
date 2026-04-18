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

        let per_crate = query_crate_deps(db).unwrap_or_default();
        let units: Vec<UnitDeps> = per_crate
            .into_iter()
            .map(|(unit_name, deps)| UnitDeps { unit_name, deps })
            .collect();

        let result = ProjectDependencies { units };
        serde_json::to_value(&result).map_err(DataProviderError::from)
    }
}
