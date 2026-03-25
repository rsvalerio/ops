//! Tokei extension: code statistics (lines of code, comments, blanks) via the tokei library.
//! Language-agnostic -- loads for any project regardless of stack.

mod ingestor;
#[cfg(test)]
mod tests;
pub mod views;

pub use ingestor::TokeiIngestor;

use ops_duckdb::{init_schema, DataIngestor, DuckDb};
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
        DataProviderSchema {
            description: "Code statistics from tokei (lines of code, comments, blanks per file)",
            fields: vec![
                DataField {
                    name: "language",
                    type_name: "str",
                    description: "Language name (e.g., Rust, Python, JavaScript)",
                },
                DataField {
                    name: "file",
                    type_name: "str",
                    description: "File path relative to workspace root",
                },
                DataField {
                    name: "code",
                    type_name: "int",
                    description: "Lines of code",
                },
                DataField {
                    name: "comments",
                    type_name: "int",
                    description: "Comment lines",
                },
                DataField {
                    name: "blanks",
                    type_name: "int",
                    description: "Blank lines",
                },
                DataField {
                    name: "lines",
                    type_name: "int",
                    description: "Total lines (code + comments + blanks)",
                },
            ],
        }
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

pub fn collect_tokei(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error> {
    let mut languages = Languages::new();
    let tokei_config = TokeiConfig::default();
    let excluded: &[&str] = &[];
    languages.get_statistics(&[working_dir], excluded, &tokei_config);

    Ok(flatten_tokei_to_json(&languages, working_dir))
}

pub fn flatten_tokei_to_json(languages: &Languages, workspace_root: &Path) -> serde_json::Value {
    let mut records = Vec::new();

    for (lang_type, language) in languages.iter() {
        let lang_name = lang_type.name();
        for report in &language.reports {
            let file_path = report
                .name
                .strip_prefix(workspace_root)
                .unwrap_or(&report.name);
            let file_str = file_path.to_string_lossy();
            let stats = &report.stats;
            records.push(serde_json::json!({
                "language": lang_name,
                "file": file_str,
                "code": stats.code,
                "comments": stats.comments,
                "blanks": stats.blanks,
                "lines": stats.lines(),
            }));
        }
    }

    serde_json::Value::Array(records)
}

pub fn load_tokei(data_dir: &Path, db: &DuckDb) -> Result<(), anyhow::Error> {
    let json_path = data_dir.join("tokei_files.json");
    if !json_path.exists() {
        anyhow::bail!("tokei_files.json not found. Run collect first.");
    }
    init_schema(db)?;
    let ingestor = TokeiIngestor;
    ingestor.load(data_dir, db)?;
    Ok(())
}
