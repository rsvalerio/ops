//! LOC, file count, and per-language queries over `tokei_files`.

use crate::DuckDb;
use std::collections::HashMap;

use ops_core::project_identity::LanguageStat;

use super::super::ingest::table_exists;
use super::helpers::{query_per_crate_i64, query_project_scalar, PerCrateI64Query};

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
        table: "tokei_files",
        member_paths,
        select_expr: "COUNT(f.file)",
        join_alias: "f",
        join_column: "file",
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
/// percentages of both. Ordered by LOC descending. Languages contributing
/// under 0.1% of total LOC are omitted.
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
            "SELECT language, \
                    SUM(code) AS loc, \
                    COUNT(*) AS files, \
                    ROUND(SUM(code) * 100.0 / NULLIF((SELECT SUM(code) FROM tokei_files), 0), 1) AS loc_pct, \
                    ROUND(COUNT(*) * 100.0 / NULLIF((SELECT COUNT(*) FROM tokei_files), 0), 1) AS files_pct \
             FROM tokei_files \
             GROUP BY language \
             ORDER BY SUM(code) DESC",
        )
        .context("preparing query_project_languages")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok(LanguageStat {
                name: row.get(0)?,
                loc: row.get(1)?,
                files: row.get(2)?,
                loc_pct: row.get(3)?,
                files_pct: row.get(4)?,
            })
        })
        .context("querying project languages")?;

    let mut languages = Vec::new();
    for row in rows {
        let stat = row.context("reading language row")?;
        if stat.loc_pct >= 0.1 {
            languages.push(stat);
        }
    }
    Ok(languages)
}

/// Query per-crate lines of code from `tokei_files`.
///
/// Returns a map of member path -> total code lines. Members with no matching
/// files get 0.
pub fn query_crate_loc(db: &DuckDb, member_paths: &[&str]) -> anyhow::Result<HashMap<String, i64>> {
    query_per_crate_i64(&PerCrateI64Query {
        db,
        table: "tokei_files",
        member_paths,
        select_expr: "COALESCE(SUM(f.code), 0)",
        join_alias: "f",
        join_column: "file",
        label: "query_crate_loc",
    })
}
