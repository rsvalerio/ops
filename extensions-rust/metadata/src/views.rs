//! SQL utilities for cargo metadata.
//!
//! # Security (SEC-001)
//!
//! Identifier gating is handled by `ops_sqlite::sql` (shared
//! defense-in-depth validation). This module only contains metadata-specific
//! SQL specs.

use ops_sqlite::sql::{CreateViewSql, JsonTableLoad, TableName};

/// Declarative load spec for `metadata.json`: the whole `cargo metadata`
/// document, nested structure intact, stored verbatim as a single JSON blob
/// row in `metadata_raw`. The [`crate_dependencies_view_sql`] view tears the
/// blob apart with JSON1 (`json_each`) at query time.
///
/// SEC-12: the table name is const-validated at construction, and the staged
/// JSON reaches the engine as a bound `?1` parameter — no path is ever
/// interpolated into a statement.
pub const METADATA_RAW_LOAD: JsonTableLoad = JsonTableLoad::single_object_blob("metadata_raw");

/// The `crate_dependencies` view over the `metadata_raw` JSON blob.
///
/// SQLite JSON1 port of the `DuckDB` `unnest` body, verified against this
/// workspace's real `cargo metadata` (see `docs/duckdb-alternatives.md`,
/// preserved in `docs/duckdb-to-sqlite.md`'s appendix): 354 rows, matching
/// per-crate counts, and the `cfg(unix)` target rows that PATTERN-1 /
/// TASK-1056 exists to preserve come through intact.
///
/// TASK-0982: include path/intra-workspace deps. Cargo metadata sets
/// `dep.source` to NULL for path dependencies, so a `WHERE dep.source IS NOT
/// NULL` filter would silently drop workspace-internal coupling — the
/// dependency count would underreport reality for workspaces (such as this
/// repo) that use path deps as the primary modularity tool.
///
/// PATTERN-1 / TASK-1056: include `dep.target` so target-conditional
/// declarations of the same dep (e.g. a `[target.'cfg(windows)'.dependencies]`
/// and a `[target.'cfg(unix)'.dependencies]` pair) preserve their
/// platform-specific shape instead of presenting as identical `(crate_name,
/// dependency_name, version_req, dependency_kind, is_optional)` tuples that
/// double-count in downstream consumers. NULL means "all targets" (the
/// default `[dependencies]` table); a non-empty string is the cfg expression.
///
/// ERR-2 / TASK-1253: surface `pkg.manifest_path` so callers
/// (e.g. `query_crate_dep_counts`) can key per-crate counts on a unique
/// identifier. `pkg.name` collides for renamed (`package = "alt"`) or
/// duplicate-named workspace crates and used to silently mis-attribute
/// counts. The column is nullable in extreme edge cases (synthetic
/// metadata) and ordered last so existing positional consumers keep
/// working.
///
/// Boolean shape: `is_optional` is INTEGER 0/1 (`COALESCE(...,0)` over
/// `json_extract`), not a SQL boolean — SQLite has none. No consumer reads
/// `is_optional` typed (checked: `query_crate_deps` selects name/req only).
///
/// SEC-12 / TASK-1864: returned as the gated [`CreateViewSql`] newtype so the
/// `crate_dependencies` DDL this crate executes is provably builder-produced,
/// matching the sibling ingestors.
#[must_use = "the view statement is the only gated form; discarding it means nothing is executed"]
pub fn crate_dependencies_view_sql() -> CreateViewSql {
    CreateViewSql::create_or_replace(
        TableName::from_static("crate_dependencies"),
        TableName::from_static("metadata_raw"),
        "WITH pkgs AS (SELECT p.value AS pkg FROM <source> m, json_each(m.json,'$.packages') p), \
     ws AS (SELECT w.value AS member_id FROM <source> m, json_each(m.json,'$.workspace_members') w), \
     member_deps AS ( \
         SELECT json_extract(pkg,'$.name') AS crate_name, \
                json_extract(pkg,'$.manifest_path') AS crate_manifest_path, \
                d.value AS dep \
         FROM pkgs, json_each(pkgs.pkg,'$.dependencies') d \
         WHERE json_extract(pkg,'$.id') IN (SELECT member_id FROM ws) \
     ) \
     SELECT crate_name, json_extract(dep,'$.name') AS dependency_name, \
            json_extract(dep,'$.req') AS version_req, \
            COALESCE(json_extract(dep,'$.kind'),'normal') AS dependency_kind, \
            COALESCE(json_extract(dep,'$.optional'),0) AS is_optional, \
            NULLIF(json_extract(dep,'$.target'),'') AS target, \
            crate_manifest_path \
     FROM member_deps \
     ORDER BY crate_name, dependency_kind, dependency_name, target",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_dependencies_view_sql_contains_expected_clauses() {
        let sql = crate_dependencies_view_sql().to_string();
        // SQLite has no CREATE OR REPLACE: the batch drops first.
        assert!(sql.contains("DROP VIEW IF EXISTS \"crate_dependencies\""));
        assert!(sql.contains("CREATE VIEW \"crate_dependencies\""));
        // JSON1 replaces DuckDB's unnest over the blob's packages array.
        assert!(sql.contains("json_each(m.json,'$.packages')"));
        assert!(sql.contains("workspace_members"));
        // TASK-0982: path/intra-workspace deps must not be filtered out.
        assert!(!sql.contains("dep.source IS NOT NULL"));
        assert!(sql.contains("dependency_kind"));
        // PATTERN-1 / TASK-1056: target column must surface so
        // target-conditional duplicates don't collapse into identical
        // tuples and inflate downstream counts.
        assert!(sql.contains("dep,'$.target'"));
        assert!(sql.contains("AS target"));
    }

    /// SEC-12: the load spec's DDL is the single-blob shape — one `json`
    /// TEXT NOT NULL column — quoted and idempotent.
    #[test]
    fn metadata_raw_load_declares_single_json_blob() {
        let sql = METADATA_RAW_LOAD.create_table_sql().to_string();
        assert!(
            sql.contains("DROP TABLE IF EXISTS \"metadata_raw\";"),
            "expected drop+create batch: {sql}"
        );
        assert!(
            sql.contains("CREATE TABLE \"metadata_raw\" (\"json\" TEXT NOT NULL)"),
            "expected single JSON blob column: {sql}"
        );
    }
}
