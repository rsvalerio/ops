//! Project- and per-crate coverage queries over `coverage_files`.

use crate::DuckDb;
use std::collections::HashMap;

use super::super::ingest::table_exists;
use super::super::validation::{validate_no_traversal, validate_path_chars};
use super::helpers::{
    collect_per_crate_map, coverage_col_select, members_cte_prefix, prepare_per_crate,
    resolve_per_crate, ColumnAlias, CrateCoverage, Resolved,
};

/// Query total coverage across the whole project from `coverage_files`.
pub fn query_project_coverage(db: &DuckDb) -> anyhow::Result<CrateCoverage> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_project_coverage")?;

    if !table_exists(&conn, "coverage_files")? {
        return Ok(CrateCoverage::zero());
    }

    let sql = format!("SELECT {} FROM coverage_files", coverage_col_select(None));
    conn.query_row(&sql, [], |row: &duckdb::Row| {
        Ok(CrateCoverage {
            lines_count: row.get(0)?,
            lines_covered: row.get(1)?,
            lines_percent: row.get(2)?,
        })
    })
    .context("querying project coverage")
}

/// Query per-crate coverage from `coverage_files`.
///
/// Returns a map of member path -> CrateCoverage. Members with no matching
/// files get zeroed coverage. Handles both absolute and relative filenames
/// from LLVM coverage output.
pub fn query_crate_coverage(
    db: &DuckDb,
    member_paths: &[&str],
    workspace_root: &str,
) -> anyhow::Result<HashMap<String, CrateCoverage>> {
    // workspace_root flows into a bound parameter (not interpolated) so SQL
    // injection is structurally impossible; validation here is layered for
    // semantic safety: reject control chars (validate_path_chars) and parent
    // traversal segments (validate_no_traversal) that would produce nonsense
    // starts_with matches and confuse coverage attribution.
    validate_path_chars(workspace_root)?;
    validate_no_traversal(std::path::Path::new(workspace_root))?;

    // SEC-12: strip a trailing '/' so the prefix join below produces a single
    // boundary slash. With a trailing slash the join would build
    // "/ws//crates/foo/" and absolute filenames "/ws/crates/foo/src/lib.rs"
    // would silently fail to match, dropping per-crate coverage to 0 with no
    // diagnostic. Done at validation time (callers may pass either form).
    let workspace_root = workspace_root.trim_end_matches('/');

    let label = "query_crate_coverage";
    let setup = prepare_per_crate(db, "coverage_files", member_paths, label)?;
    let (conn, placeholders, mut paths) =
        match resolve_per_crate(setup, member_paths, CrateCoverage::zero) {
            Resolved::Done(map) => return Ok(map),
            Resolved::Continue(conn, placeholders, paths) => (conn, placeholders, paths),
        };

    // workspace_root is the last bound parameter (? after VALUES placeholders)
    paths.push(workspace_root.to_string());

    // LLVM coverage filenames may be either:
    //   1. relative to workspace_root (e.g., "crates/foo/src/lib.rs")
    //   2. absolute (e.g., "/abs/workspace_root/crates/foo/src/lib.rs")
    // The OR matches both shapes against the same member path. The trailing '/'
    // ensures a member "crates/foo" does not match "crates/foobar/...".
    let cte = members_cte_prefix(&placeholders);
    // SEC-12: the join alias `c` is constructed via the validated newtype so
    // its presence in the formatted SQL cannot be a regression vector.
    let join_alias = ColumnAlias::new("c").expect("static alias 'c' is a valid identifier");
    let sql = format!(
        "{cte} \
         SELECT m.path, {} \
         FROM members m \
         LEFT JOIN coverage_files c \
             ON starts_with(c.filename, m.path || '/') \
             OR starts_with(c.filename, ? || '/' || m.path || '/') \
         GROUP BY m.path",
        coverage_col_select(Some(&join_alias))
    );

    collect_per_crate_map(&conn, &sql, label, &paths, |row| {
        Ok((
            row.get::<_, String>(0)?,
            CrateCoverage {
                lines_count: row.get(1)?,
                lines_covered: row.get(2)?,
                lines_percent: row.get(3)?,
            },
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DuckDb;

    fn setup_coverage_table(db: &DuckDb, rows: &[(&str, i64, i64)]) {
        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE coverage_files (filename VARCHAR, lines_count BIGINT, lines_covered BIGINT)",
        )
        .expect("create");
        for (filename, count, covered) in rows {
            conn.execute(
                "INSERT INTO coverage_files VALUES (?, ?, ?)",
                duckdb::params![filename, count, covered],
            )
            .expect("insert");
        }
    }

    #[test]
    fn dual_prefix_matches_relative_filenames() {
        let db = DuckDb::open_in_memory().expect("db");
        // Relative filenames (no workspace_root prefix)
        setup_coverage_table(
            &db,
            &[
                ("crates/foo/src/lib.rs", 100, 80),
                ("crates/foo/src/util.rs", 50, 25),
                ("crates/bar/src/lib.rs", 10, 1),
            ],
        );
        let result = query_crate_coverage(&db, &["crates/foo"], "/workspace").expect("query ok");
        let foo = result.get("crates/foo").expect("foo present");
        assert_eq!(foo.lines_count, 150);
        assert_eq!(foo.lines_covered, 105);
    }

    #[test]
    fn dual_prefix_matches_absolute_filenames() {
        let db = DuckDb::open_in_memory().expect("db");
        // Absolute filenames including workspace_root
        setup_coverage_table(
            &db,
            &[
                ("/ws/root/crates/foo/src/lib.rs", 200, 100),
                ("/ws/root/crates/bar/src/lib.rs", 10, 0),
            ],
        );
        let result = query_crate_coverage(&db, &["crates/foo"], "/ws/root").expect("query ok");
        let foo = result.get("crates/foo").expect("foo present");
        assert_eq!(foo.lines_count, 200);
        assert_eq!(foo.lines_covered, 100);
    }

    #[test]
    fn dual_prefix_does_not_double_count_when_both_match() {
        let db = DuckDb::open_in_memory().expect("db");
        // A pathological row matching both branches would otherwise be counted
        // once: starts_with(filename, "crates/foo/") matches relatively.
        // Filename is relative, so only the first branch matches.
        setup_coverage_table(&db, &[("crates/foo/src/lib.rs", 100, 50)]);
        let result = query_crate_coverage(&db, &["crates/foo"], "/ws").expect("query ok");
        let foo = result.get("crates/foo").expect("foo present");
        assert_eq!(foo.lines_count, 100);
        assert_eq!(foo.lines_covered, 50);
    }

    /// SEC-12: workspace_root with and without a trailing '/' must yield
    /// identical results. Before normalization a trailing '/' produced
    /// "/ws//crates/foo/" in the prefix join and silently zeroed coverage.
    #[test]
    fn workspace_root_trailing_slash_yields_same_results() {
        let db_a = DuckDb::open_in_memory().expect("db a");
        let db_b = DuckDb::open_in_memory().expect("db b");
        let rows = [
            ("/ws/root/crates/foo/src/lib.rs", 200, 100),
            ("/ws/root/crates/bar/src/lib.rs", 10, 0),
        ];
        setup_coverage_table(&db_a, &rows);
        setup_coverage_table(&db_b, &rows);

        let plain = query_crate_coverage(&db_a, &["crates/foo"], "/ws/root").expect("plain ok");
        let trailing =
            query_crate_coverage(&db_b, &["crates/foo"], "/ws/root/").expect("trailing ok");

        let plain_foo = plain.get("crates/foo").expect("plain foo");
        let trailing_foo = trailing.get("crates/foo").expect("trailing foo");
        assert_eq!(plain_foo.lines_count, trailing_foo.lines_count);
        assert_eq!(plain_foo.lines_covered, trailing_foo.lines_covered);
        assert_eq!(plain_foo.lines_count, 200);
        assert_eq!(plain_foo.lines_covered, 100);
    }

    #[test]
    fn dual_prefix_excludes_sibling_with_shared_prefix() {
        let db = DuckDb::open_in_memory().expect("db");
        setup_coverage_table(&db, &[("crates/foobar/src/lib.rs", 100, 50)]);
        let result = query_crate_coverage(&db, &["crates/foo"], "/ws").expect("query ok");
        let foo = result.get("crates/foo").expect("foo present");
        assert_eq!(foo.lines_count, 0, "trailing slash boundary preserved");
    }
}
