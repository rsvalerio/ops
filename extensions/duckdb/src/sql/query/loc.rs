//! LOC, file count, and per-language queries over `tokei_files`.

use crate::DuckDb;
use std::collections::HashMap;

use ops_core::project_identity::LanguageStat;

use super::super::ingest::table_exists;
use super::helpers::{
    query_per_crate_i64, query_project_scalar, ColumnAlias, ColumnName, PerCrateI64Query, TableName,
};

/// Query total file count across the whole project from `tokei_files`.
pub fn query_project_file_count(db: &DuckDb) -> anyhow::Result<i64> {
    query_project_scalar(
        db,
        "tokei_files",
        "SELECT COUNT(*) FROM tokei_files",
        "query_project_file_count",
    )
}

/// Query per-crate file counts from `tokei_files`.
///
/// Returns a map of member path -> file count. Members with no matching
/// files get 0.
pub fn query_crate_file_count(
    db: &DuckDb,
    member_paths: &[&str],
) -> anyhow::Result<HashMap<String, i64>> {
    query_per_crate_i64(&PerCrateI64Query {
        db,
        table: TableName::new("tokei_files")?,
        member_paths,
        select_expr: "COUNT(f.file)",
        join_alias: ColumnAlias::new("f")?,
        join_column: ColumnName::new("file")?,
        label: "query_crate_file_count",
    })
}

/// Query total lines of code across the whole project from `tokei_files`.
pub fn query_project_loc(db: &DuckDb) -> anyhow::Result<i64> {
    query_project_scalar(
        db,
        "tokei_files",
        "SELECT COALESCE(SUM(code), 0) FROM tokei_files",
        "query_project_loc",
    )
}

/// Query per-language breakdown from `tokei_files`: LOC, file count, and
/// percentages of both. Ordered by LOC descending.
///
/// READ-5 / TASK-0362: languages whose `loc_pct` rounds below 0.1% are
/// omitted, *including* the case where every language is sub-threshold.
/// Previously this function fell back to the top entry when the filtered
/// set would otherwise be empty, which contradicted the documented
/// "omit < 0.1%" contract and made it impossible for callers to
/// distinguish "no tokei data" from "every language tiny". The empty
/// return is now the only signal, matching the doc.
pub fn query_project_languages(db: &DuckDb) -> anyhow::Result<Vec<LanguageStat>> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_project_languages")?;

    if !table_exists(&conn, "tokei_files")? {
        return Ok(vec![]);
    }

    let mut stmt = conn
        .prepare(
            "WITH totals AS (SELECT COALESCE(SUM(code), 0) AS total_loc, COUNT(*) AS total_files FROM tokei_files) \
             SELECT language, \
                    SUM(code) AS loc, \
                    COUNT(*) AS files, \
                    ROUND(SUM(code) * 100.0 / NULLIF(totals.total_loc, 0), 1) AS loc_pct, \
                    ROUND(COUNT(*) * 100.0 / NULLIF(totals.total_files, 0), 1) AS files_pct \
             FROM tokei_files, totals \
             GROUP BY language, totals.total_loc, totals.total_files \
             ORDER BY SUM(code) DESC",
        )
        .context("preparing query_project_languages")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok(LanguageStat::new(
                row.get::<_, String>(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .context("querying project languages")?;

    let mut filtered = Vec::new();
    for row in rows {
        let stat = row.context("reading language row")?;
        if stat.loc_pct >= 0.1 {
            filtered.push(stat);
        }
    }
    Ok(filtered)
}

/// Query per-crate lines of code from `tokei_files`.
///
/// Returns a map of member path -> total code lines. Members with no matching
/// files get 0.
pub fn query_crate_loc(db: &DuckDb, member_paths: &[&str]) -> anyhow::Result<HashMap<String, i64>> {
    query_per_crate_i64(&PerCrateI64Query {
        db,
        table: TableName::new("tokei_files")?,
        member_paths,
        select_expr: "COALESCE(SUM(f.code), 0)",
        join_alias: ColumnAlias::new("f")?,
        join_column: ColumnName::new("file")?,
        label: "query_crate_loc",
    })
}
