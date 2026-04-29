//! Tokei extension: code statistics (lines of code, comments, blanks) via the tokei library.
//! Language-agnostic -- loads for any project regardless of stack.

mod ingestor;
#[cfg(test)]
mod tests;
pub mod views;

pub use ingestor::TokeiIngestor;

use ops_duckdb::DuckDb;
use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use std::path::Path;
use tokei::{Config as TokeiConfig, Languages};

pub const NAME: &str = "tokei";
#[allow(dead_code)]
pub const DESCRIPTION: &str = "Code statistics provider (lines of code, comments, blanks)";
#[allow(dead_code)]
pub const SHORTNAME: &str = "tokei";
pub const DATA_PROVIDER_NAME: &str = "tokei";

pub struct TokeiExtension;

ops_extension::impl_extension! {
    TokeiExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(TokeiProvider));
    },
    factory: TOKEI_FACTORY = |_, _| {
        Some((NAME, Box::new(TokeiExtension)))
    },
}

struct TokeiProvider;

impl DataProvider for TokeiProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        ops_duckdb::try_provide_from_db(ctx, provide_from_db, |ctx| {
            collect_tokei(&ctx.working_directory)
        })
    }

    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema::new(
            "Code statistics from tokei (lines of code, comments, blanks per file)",
            vec![
                DataField::new(
                    "language",
                    "str",
                    "Language name (e.g., Rust, Python, JavaScript)",
                ),
                DataField::new("file", "str", "File path relative to workspace root"),
                DataField::new("code", "int", "Lines of code"),
                DataField::new("comments", "int", "Comment lines"),
                DataField::new("blanks", "int", "Blank lines"),
                DataField::new("lines", "int", "Total lines (code + comments + blanks)"),
            ],
        )
    }
}

fn query_tokei_files(db: &DuckDb) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::query_rows_to_json(
        db,
        "SELECT language, file, code, comments, blanks, lines FROM tokei_files",
        |row| {
            Ok(serde_json::json!({
                "language": row.get::<_, String>(0)?,
                "file": row.get::<_, String>(1)?,
                "code": row.get::<_, i64>(2)?,
                "comments": row.get::<_, i64>(3)?,
                "blanks": row.get::<_, i64>(4)?,
                "lines": row.get::<_, i64>(5)?,
            }))
        },
    )
}

fn provide_from_db(db: &DuckDb, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::provide_via_ingestor(db, ctx, "tokei_files", &TokeiIngestor, query_tokei_files)
}

/// Directories excluded from `cargo`-style projects' tokei scan.
///
/// Build artifacts and VCS directories produce nonsense LOC counts and slow
/// the scan. Tokei's defaults already skip vendored deps, but it does not
/// skip e.g. `target/` for Rust or `node_modules` for JS unless asked.
pub const TOKEI_DEFAULT_EXCLUDED: &[&str] = &[
    "target",
    ".git",
    "node_modules",
    ".venv",
    "venv",
    "dist",
    "build",
];

pub fn collect_tokei(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error> {
    let mut languages = Languages::new();
    let tokei_config = TokeiConfig::default();
    languages.get_statistics(&[working_dir], TOKEI_DEFAULT_EXCLUDED, &tokei_config);

    Ok(flatten_tokei_to_json(&languages, working_dir))
}

pub fn flatten_tokei_to_json(languages: &Languages, workspace_root: &Path) -> serde_json::Value {
    let records: Vec<serde_json::Value> = languages
        .iter()
        .flat_map(|(lang_type, language)| {
            language
                .reports
                .iter()
                .map(move |report| report_to_json(lang_type.name(), report, workspace_root))
        })
        .collect();
    serde_json::Value::Array(records)
}

fn report_to_json(
    language: &str,
    report: &tokei::Report,
    workspace_root: &Path,
) -> serde_json::Value {
    let file_str = relativize_path(&report.name, workspace_root);
    let stats = &report.stats;
    serde_json::json!({
        "language": language,
        "file": file_str,
        "code": stats.code,
        "comments": stats.comments,
        "blanks": stats.blanks,
        "lines": stats.lines(),
    })
}

/// Render a tokei `Report.name` path as a workspace-relative UTF-8 string.
///
/// READ-5 (TASK-0504): this is intentionally lossy. The DuckDB `tokei_files`
/// view that consumes this column is read-only at the value level (it never
/// round-trips the path back to disk), so corrupting an invalid UTF-8 byte
/// to `U+FFFD` only affects display and join-by-string-prefix attribution.
/// The strict `DbError::NonUtf8Path` policy used by `upsert_data_source`
/// applies to **paths interpolated into SQL** — the `tokei_files` view is
/// populated from a JSON sidecar, not from a SQL string literal, so the
/// risks differ. The trade-off is recorded here so future refactors stop
/// at this comment instead of "fixing" the lossy call.
fn relativize_path(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}
