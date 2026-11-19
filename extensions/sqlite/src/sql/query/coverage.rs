//! Project- and per-crate coverage queries over `coverage_files`.

use crate::Sqlite;
use std::collections::HashMap;

use super::super::validation::{validate_no_traversal, validate_path_chars};
use super::helpers::{
    collect_per_crate_map, coverage_col_select, member_owns_file, members_cte_prefix,
    prepare_per_crate, query_project_row, resolve_per_crate, ColumnAlias, CrateCoverage, QuerySpec,
    Resolved,
};

/// Join alias of the per-crate coverage query, validated at build time.
const COVERAGE_ALIAS: ColumnAlias = ColumnAlias::from_static("c");

/// Query total coverage across the whole project from `coverage_files`.
///
/// Shares the lock + `table_exists` + `query_row` + `with_context`
/// prologue with `query_project_scalar` via the generalised
/// `query_project_row` helper. Both project-scalar (LOC, deps) and
/// project-row (coverage) callers flow through the same scaffolding.
///
/// # Errors
///
/// If the database lock is poisoned, or the query or row decode fails. A
/// missing `coverage_files` table is not an error.
pub fn query_project_coverage(db: &Sqlite) -> anyhow::Result<CrateCoverage> {
    let sql = format!("SELECT {} FROM coverage_files", coverage_col_select(None));
    query_project_row(
        db,
        &QuerySpec { table: "coverage_files", sql: &sql, label: "query_project_coverage" },
        CrateCoverage::zero(),
        |row| Ok(CrateCoverage::new(row.get(0)?, row.get(1)?, row.get(2)?)),
    )
}

/// Query per-crate coverage from `coverage_files`.
///
/// Returns a map of member path -> `CrateCoverage`. Members with no matching
/// files get zeroed coverage. Handles both absolute and relative filenames
/// from LLVM coverage output. Each file counts toward exactly one member,
/// the one with the longest matching path prefix, so nested members do not
/// double-count.
///
/// # Errors
///
/// If `workspace_root` contains control characters or parent-traversal
/// segments, if the database lock is poisoned, or if the query or row decode
/// fails.
pub fn query_crate_coverage(
    db: &Sqlite,
    member_paths: &[&str],
    workspace_root: &str,
) -> anyhow::Result<HashMap<String, CrateCoverage>> {
    // workspace_root flows into a bound parameter (not interpolated), so SQL
    // injection is structurally impossible and any character a real directory
    // name can hold is accepted. The checks are semantic only: control
    // characters and parent-traversal segments would produce nonsense prefix
    // matches and confuse coverage attribution.
    validate_path_chars(workspace_root)?;
    validate_no_traversal(std::path::Path::new(workspace_root))?;

    // Strip a trailing '/' so the prefix join below produces a single
    // boundary slash. With a trailing slash the join would build
    // "/ws//crates/foo/" and absolute filenames "/ws/crates/foo/src/lib.rs"
    // would silently fail to match, dropping per-crate coverage to 0 with no
    // diagnostic. Done at validation time (callers may pass either form).
    let workspace_root = workspace_root.trim_end_matches('/');

    let label = "query_crate_coverage";
    let setup = prepare_per_crate(db, "coverage_files", member_paths, label)?;
    let (conn, placeholders) = match resolve_per_crate(setup, member_paths, CrateCoverage::zero) {
        Resolved::Done(map) => return Ok(map),
        Resolved::Continue(conn, placeholders) => (conn, placeholders),
    };

    // LLVM coverage filenames may be either:
    //   1. relative to workspace_root (e.g., "crates/foo/src/lib.rs")
    //   2. absolute (e.g., "/abs/workspace_root/crates/foo/src/lib.rs")
    // The derived table strips the workspace-root prefix from the absolute
    // shape, so both reach the member join as one relative `rel` column.
    // `:root` is a named parameter: every occurrence binds the one value,
    // which takes the index after the VALUES placeholders.
    let cte = members_cte_prefix(&placeholders);
    let owns = member_owns_file(&format!("{COVERAGE_ALIAS}.rel"));
    let sql = format!(
        "{cte} \
         SELECT m.path, {cols} \
         FROM members m \
         LEFT JOIN ( \
             SELECT CASE \
                 WHEN substr(filename, 1, length(:root || '/')) = :root || '/' \
                 THEN substr(filename, length(:root || '/') + 1) \
                 ELSE filename END AS rel, \
                 lines_count, lines_covered \
             FROM coverage_files \
         ) {COVERAGE_ALIAS} ON {owns} \
         GROUP BY m.path",
        cols = coverage_col_select(Some(&COVERAGE_ALIAS))
    );

    let params = member_paths.iter().copied().chain(std::iter::once(workspace_root));

    collect_per_crate_map(&conn, &sql, label, params, |row| {
        Ok((row.get::<_, String>(0)?, CrateCoverage::new(row.get(1)?, row.get(2)?, row.get(3)?)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Sqlite;

    fn setup_coverage_table(db: &Sqlite, rows: &[(&str, i64, i64)]) {
        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE coverage_files (filename VARCHAR, lines_count BIGINT, lines_covered BIGINT)",
        )
        .expect("create");
        for (filename, count, covered) in rows {
            conn.execute(
                "INSERT INTO coverage_files VALUES (?, ?, ?)",
                rusqlite::params![filename, count, covered],
            )
            .expect("insert");
        }
    }

    #[test]
    fn dual_prefix_matches_relative_filenames() {
        let db = Sqlite::open_in_memory().expect("db");
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
        let db = Sqlite::open_in_memory().expect("db");
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
        let db = Sqlite::open_in_memory().expect("db");
        // A pathological row matching both branches would otherwise be counted
        // once: starts_with(filename, "crates/foo/") matches relatively.
        // Filename is relative, so only the first branch matches.
        setup_coverage_table(&db, &[("crates/foo/src/lib.rs", 100, 50)]);
        let result = query_crate_coverage(&db, &["crates/foo"], "/ws").expect("query ok");
        let foo = result.get("crates/foo").expect("foo present");
        assert_eq!(foo.lines_count, 100);
        assert_eq!(foo.lines_covered, 50);
    }

    /// `workspace_root` with and without a trailing '/' must yield
    /// identical results. Before normalization a trailing '/' produced
    /// "/ws//crates/foo/" in the prefix join and silently zeroed coverage.
    #[test]
    fn workspace_root_trailing_slash_yields_same_results() {
        let db_a = Sqlite::open_in_memory().expect("db a");
        let db_b = Sqlite::open_in_memory().expect("db b");
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
        let db = Sqlite::open_in_memory().expect("db");
        setup_coverage_table(&db, &[("crates/foobar/src/lib.rs", 100, 50)]);
        let result = query_crate_coverage(&db, &["crates/foo"], "/ws").expect("query ok");
        let foo = result.get("crates/foo").expect("foo present");
        assert_eq!(foo.lines_count, 0, "trailing slash boundary preserved");
    }

    /// A file under a nested member counts only toward the innermost member,
    /// in both filename shapes, so per-crate totals sum to the project total.
    #[test]
    fn nested_members_attribute_each_file_once() {
        let db = Sqlite::open_in_memory().expect("db");
        setup_coverage_table(
            &db,
            &[("a/src/lib.rs", 10, 5), ("a/b/src/lib.rs", 20, 10), ("/ws/a/b/src/x.rs", 4, 4)],
        );
        let result = query_crate_coverage(&db, &["a", "a/b"], "/ws").expect("query ok");
        assert_eq!(result["a"].lines_count, 10, "a/b's files must not count toward a");
        assert_eq!(result["a/b"].lines_count, 24);
        assert_eq!(result["a/b"].lines_covered, 14);
        let total = query_project_coverage(&db).expect("project");
        assert_eq!(result.values().map(|c| c.lines_count).sum::<i64>(), total.lines_count);
    }

    /// Paths are bound parameters, so characters that are legal in a
    /// directory name (parentheses, `+`, `@`, spaces, non-ASCII) must not turn
    /// the query into an error and silently drop per-crate coverage.
    #[test]
    fn paths_with_punctuation_and_non_ascii_are_queried_not_rejected() {
        let db = Sqlite::open_in_memory().expect("db");
        let root = "/home/u/My Project (old)+x/@scope/проект";
        let member = "crates/foo (v2)+@é";
        setup_coverage_table(
            &db,
            &[
                (&format!("{root}/{member}/src/lib.rs"), 200, 100),
                (&format!("{member}/src/util.rs"), 50, 25),
                (&format!("{root}/crates/bar/src/lib.rs"), 10, 0),
            ],
        );
        let result = query_crate_coverage(&db, &[member], root).expect("query ok");
        let cov = result.get(member).expect("member present");
        assert_eq!(cov.lines_count, 250);
        assert_eq!(cov.lines_covered, 125);
    }

    /// Control characters stay rejected in both the workspace root and
    /// a member path.
    #[test]
    fn control_characters_in_paths_are_still_rejected() {
        let db = Sqlite::open_in_memory().expect("db");
        setup_coverage_table(&db, &[("crates/foo/src/lib.rs", 1, 1)]);
        assert!(query_crate_coverage(&db, &["crates/foo"], "/ws\nroot").is_err());
        assert!(query_crate_coverage(&db, &["crates/\u{1}foo"], "/ws").is_err());
    }
}
