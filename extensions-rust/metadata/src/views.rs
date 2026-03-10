//! SQL utilities for cargo metadata.
//!
//! # Security (SEC-001)
//!
//! Path validation and SQL escaping are handled by `cargo_ops_duckdb::sql`
//! (shared defense-in-depth validation). This module only contains
//! metadata-specific SQL generation.

use cargo_ops_duckdb::sql::SqlError;
use std::path::Path;

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
            COALESCE(dep.optional, false) AS is_optional \
     FROM member_deps \
     WHERE dep.source IS NOT NULL \
     ORDER BY crate_name, dependency_kind, dependency_name"
        .to_string()
}

pub fn metadata_raw_create_sql(path: &Path) -> Result<String, SqlError> {
    cargo_ops_duckdb::sql::create_table_from_json_sql(
        "metadata_raw",
        path,
        Some("maximum_object_size=67108864"),
    )
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
        assert!(sql.contains("dep.source IS NOT NULL"));
        assert!(sql.contains("dependency_kind"));
    }

    cargo_ops_duckdb::test_create_sql_validation!(metadata_raw_create_sql, "metadata.json");
}
