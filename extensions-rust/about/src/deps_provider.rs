//! Rust `project_dependencies` data provider.
//!
//! Queries `DuckDB` for per-crate direct dependencies via cargo metadata.

use ops_core::project_identity::{ProjectDependencies, UnitDeps};
use ops_duckdb::sql::{query_crate_deps, query_or_warn};
use ops_extension::{Context, DataProvider, DataProviderError};

pub const PROVIDER_NAME: &str = "project_dependencies";

pub struct RustDepsProvider;

impl DataProvider for RustDepsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let Some(db) = ops_duckdb::get_db(ctx) else {
            return Ok(serde_json::to_value(ProjectDependencies::default())?);
        };

        // ERR-2 / TASK-0376: a DuckDB schema/migration error here used to
        // surface as an empty deps list with no signal. `query_or_warn`
        // routes the failure through tracing::warn before falling back.
        let per_crate = query_or_warn(
            "query_crate_deps",
            "project_dependencies will be empty",
            std::collections::HashMap::<String, Vec<(String, String)>>::new(),
            || query_crate_deps(db),
        );
        let units: Vec<UnitDeps> = per_crate
            .into_iter()
            .map(|(unit_name, deps)| UnitDeps::new(unit_name, deps))
            .collect();

        let result = ProjectDependencies::new(units);
        serde_json::to_value(&result).map_err(DataProviderError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{RustDepsProvider, PROVIDER_NAME};
    use ops_about::test_support::capture_tracing;
    use ops_duckdb::DuckDb;
    use ops_extension::{Context, DataProvider};
    use std::sync::Arc;

    /// TEST-5 / TASK-1776 AC #2.
    #[test]
    fn provider_name_matches_the_registered_constant() {
        assert_eq!(RustDepsProvider.name(), PROVIDER_NAME);
        assert_eq!(PROVIDER_NAME, "project_dependencies");
    }

    fn provide_with(db: Option<DuckDb>) -> (serde_json::Value, String) {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        if let Some(db) = db {
            ctx.attach_db(Arc::new(db));
        }
        let (logs, value) = capture_tracing(tracing::Level::WARN, || {
            RustDepsProvider.provide(&mut ctx).expect("provide")
        });
        (value, logs)
    }

    /// TEST-5 / TASK-1776 AC #1: no `DuckDB` in the context serialises a
    /// default (empty but well-formed) `ProjectDependencies`, not an error.
    #[test]
    fn provide_without_duckdb_yields_empty_dependencies() {
        let (value, logs) = provide_with(None);
        assert_eq!(
            value.get("units").and_then(|u| u.as_array()).map(Vec::len),
            Some(0),
            "expected an empty units list, got: {value}"
        );
        assert!(
            logs.is_empty(),
            "an absent DuckDB is not a degraded mode; no warn expected, got: {logs}"
        );
    }

    /// TEST-5 / TASK-1776 AC #1: the ERR-2 / TASK-0376 contract — a `DuckDB`
    /// schema/migration error must warn before falling back, not surface as a
    /// silently empty deps list. The seeded `crate_dependencies` table is
    /// missing the `dependency_name`, `version_req` and `dependency_kind`
    /// columns the query selects, so the prepare fails while `table_exists`
    /// still passes.
    #[test]
    fn provide_warns_and_falls_back_when_the_query_fails() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE crate_dependencies (crate_name VARCHAR); \
                 INSERT INTO crate_dependencies VALUES ('a');",
            )
            .expect("seed broken-schema crate_dependencies");
        }
        let (value, logs) = provide_with(Some(db));
        assert_eq!(
            value.get("units").and_then(|u| u.as_array()).map(Vec::len),
            Some(0),
            "the fallback must be a valid empty ProjectDependencies, got: {value}"
        );
        assert!(
            logs.contains("query=\"query_crate_deps\""),
            "the failure must warn before falling back, got: {logs}"
        );
    }

    /// TEST-5 / TASK-1776 AC #1: a successful multi-crate result is mapped
    /// into one `UnitDeps` per crate, carrying `(name, version_req)` pairs.
    /// Only `dependency_kind = 'normal'` rows are included.
    #[test]
    fn provide_maps_multi_crate_rows_into_unit_deps() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE crate_dependencies (\
                    crate_name VARCHAR, \
                    dependency_name VARCHAR, \
                    version_req VARCHAR, \
                    dependency_kind VARCHAR\
                 ); \
                 INSERT INTO crate_dependencies VALUES \
                    ('alpha', 'serde', '1.0', 'normal'), \
                    ('alpha', 'anyhow', '1.0', 'normal'), \
                    ('beta', 'tracing', '0.1', 'normal'), \
                    ('beta', 'tempfile', '3', 'dev');",
            )
            .expect("seed crate_dependencies");
        }
        let (value, logs) = provide_with(Some(db));
        assert!(logs.is_empty(), "a healthy query must not warn: {logs}");

        let units = value
            .get("units")
            .and_then(|u| u.as_array())
            .expect("units");
        assert_eq!(units.len(), 2, "one UnitDeps per crate, got: {value}");
        let by_name: std::collections::BTreeMap<&str, Vec<&str>> = units
            .iter()
            .map(|u| {
                let name = u
                    .get("unit_name")
                    .and_then(|n| n.as_str())
                    .expect("unit name");
                let deps = u
                    .get("deps")
                    .and_then(|d| d.as_array())
                    .expect("deps array")
                    .iter()
                    .filter_map(|d| d.get(0).and_then(|n| n.as_str()))
                    .collect();
                (name, deps)
            })
            .collect();
        assert_eq!(by_name.get("alpha"), Some(&vec!["anyhow", "serde"]));
        assert_eq!(
            by_name.get("beta"),
            Some(&vec!["tracing"]),
            "dev-dependencies must be excluded"
        );
    }
}
