//! SQL utilities for cargo metadata.
//!
//! # Security (SEC-001)
//!
//! Path validation and SQL escaping are handled by `ops_duckdb::sql`
//! (shared defense-in-depth validation). This module only contains
//! metadata-specific SQL generation.

use ops_duckdb::sql::SqlError;
use std::path::Path;

// TASK-0982: include path/intra-workspace deps. Cargo metadata sets
// `dep.source` to NULL for path dependencies, so a `WHERE dep.source IS NOT
// NULL` filter would silently drop workspace-internal coupling — the
// dependency count would underreport reality for workspaces (such as this
// repo) that use path deps as the primary modularity tool.
//
// PATTERN-1 / TASK-1056: include `dep.target` so target-conditional
// declarations of the same dep (e.g. `[target.'cfg(windows)'.dependencies]`
// + `[target.'cfg(unix)'.dependencies]`) preserve their platform-specific
// shape instead of presenting as identical `(crate_name, dependency_name,
// version_req, dependency_kind, is_optional)` tuples that double-count in
// downstream consumers. NULL means "all targets" (the default
// `[dependencies]` table); a non-empty string is the cfg expression.
pub fn crate_dependencies_view_sql() -> String {
    "CREATE OR REPLACE VIEW crate_dependencies AS \
     WITH pkgs AS (SELECT unnest(packages) AS pkg FROM metadata_raw), \
     ws_ids AS (SELECT unnest(workspace_members) AS member_id FROM metadata_raw), \
     member_deps AS ( \
         SELECT pkg.name AS crate_name, unnest(pkg.dependencies) AS dep \
         FROM pkgs WHERE pkg.id IN (SELECT member_id FROM ws_ids) \
     ) \
     SELECT crate_name, dep.name AS dependency_name, dep.req AS version_req, \
            COALESCE(dep.kind, 'normal') AS dependency_kind, \
            COALESCE(dep.optional, false) AS is_optional, \
            NULLIF(dep.target, '') AS target \
     FROM member_deps \
     ORDER BY crate_name, dependency_kind, dependency_name, target"
        .to_string()
}

/// ARCH-9 / TASK-1247: thread the `metadata_max_bytes()` knob into the
/// DuckDB `read_json` options so the env-knob surface and the ingest-side
/// `maximum_object_size` cap share one source of truth. The previous shape
/// hardcoded 64 MiB on the DuckDB side; raising
/// `OPS_METADATA_MAX_BYTES` in the post-ingest reader did not save memory
/// because DuckDB still buffered up to the hardcoded ceiling during
/// ingest, and lowering it from the env left ingest unbounded.
pub fn metadata_raw_create_sql(path: &Path) -> Result<String, SqlError> {
    metadata_raw_create_sql_with_cap(path, crate::metadata_max_bytes())
}

/// ARCH-9 / TASK-1247: testable variant of [`metadata_raw_create_sql`] that
/// takes the cap explicitly. Production callers go through
/// `metadata_raw_create_sql`; this exists so tests can pin the
/// env-knob → SQL-option propagation without depending on the
/// `metadata_max_bytes` `OnceLock` (which is process-global and cannot be
/// re-initialised).
pub(crate) fn metadata_raw_create_sql_with_cap(path: &Path, cap: u64) -> Result<String, SqlError> {
    let opts = format!("maximum_object_size={cap}");
    ops_duckdb::sql::create_table_from_json_sql("metadata_raw", path, Some(&opts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_dependencies_view_sql_contains_expected_clauses() {
        let sql = crate_dependencies_view_sql();
        assert!(sql.contains("CREATE OR REPLACE VIEW crate_dependencies"));
        assert!(sql.contains("unnest(packages)"));
        assert!(sql.contains("workspace_members"));
        // TASK-0982: path/intra-workspace deps must not be filtered out.
        assert!(!sql.contains("dep.source IS NOT NULL"));
        assert!(sql.contains("dependency_kind"));
        // PATTERN-1 / TASK-1056: target column must surface so
        // target-conditional duplicates don't collapse into identical
        // tuples and inflate downstream counts.
        assert!(sql.contains("dep.target"));
        assert!(sql.contains("AS target"));
    }

    ops_duckdb::test_create_sql_validation!(metadata_raw_create_sql, "metadata.json");

    /// ARCH-9 / TASK-1247 AC #3: a non-default cap must propagate into the
    /// DuckDB `maximum_object_size` option so a single env knob governs
    /// both the post-ingest payload check and the ingest-time buffer
    /// ceiling.
    #[test]
    fn metadata_raw_create_sql_threads_cap_into_max_object_size() {
        let p = Path::new("metadata.json");
        let sql = metadata_raw_create_sql_with_cap(p, 9_999_999).expect("sql builds");
        assert!(
            sql.contains("maximum_object_size=9999999"),
            "raised cap must reach DuckDB ingest options: {sql}"
        );
    }
}
