//! Rust LOC extension: production / test / example line counts with doc
//! comments split from ordinary comments.
//!
//! Complements the language-agnostic `tokei` extension rather than
//! replacing it. `tokei_files` remains the source of truth for
//! cross-language totals; this crate adds the Rust-only breakdown that
//! `tokei` has no model for, since `#[cfg(test)]` blocks live in the
//! same file as the code they exercise.

pub mod counter;
mod ingestor;
#[cfg(test)]
mod tests;
pub mod views;

pub use ingestor::RustLocIngestor;

use std::path::Path;

use ignore::{DirEntry, WalkBuilder};
use ops_duckdb::DuckDb;
use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};

use counter::{count_source, region_from_path, FileCounts};

pub const NAME: &str = "rust-loc";
pub const DESCRIPTION: &str = "Rust line counts split into production, test, and example code";
pub const SHORTNAME: &str = "rust-loc";
pub const DATA_PROVIDER_NAME: &str = "rust-loc";

pub struct RustLocExtension;

ops_extension::impl_extension! {
    RustLocExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(RustLocProvider));
    },
    factory: RUST_LOC_FACTORY = |_, _| {
        Some((NAME, Box::new(RustLocExtension)))
    },
}

struct RustLocProvider;

impl DataProvider for RustLocProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        ops_duckdb::try_provide_from_db(ctx, provide_from_db, |ctx| {
            collect_rust_loc(&ctx.working_directory)
        })
    }

    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema::new(
            "Rust line counts per file and region (production, test, example)",
            vec![
                DataField::new("file", "str", "File path relative to workspace root"),
                DataField::new("region", "str", "One of: main, test, example"),
                DataField::new("code", "int", "Lines of code"),
                DataField::new("docs", "int", "Doc comment lines (/// //! /** /*!)"),
                DataField::new("comments", "int", "Ordinary comment lines"),
                DataField::new("blanks", "int", "Blank lines"),
                DataField::new("lines", "int", "Total lines in this region"),
            ],
        )
    }
}

/// Directories that never hold first-party Rust source.
///
/// `ignore::Walk` already honours `.gitignore` (which excludes `target/`
/// in every cargo project) and skips hidden entries, but only inside a
/// git repository. Listing the build directory explicitly keeps the
/// counts sane when `ops` runs on an unversioned checkout, mirroring the
/// `TOKEI_DEFAULT_EXCLUDED` policy in the tokei extension.
pub(crate) const EXCLUDED_DIRS: &[&str] = &["target", ".git"];

/// Walk `working_dir` for `.rs` files and classify each one.
///
/// Anything that cannot be read — an individual file, or a whole
/// subtree the walker cannot descend — is logged and skipped rather
/// than aborting the scan: a partial count is more useful than none,
/// and an unreadable path is not a data-integrity problem for a
/// display-only statistic. Both failure modes warn, so a silently
/// short count is always accompanied by a log line.
pub fn collect_rust_loc(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error> {
    let mut records = Vec::new();

    let walker = WalkBuilder::new(working_dir)
        .filter_entry(|entry| !is_excluded_dir(entry))
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                tracing::warn!(%error, "rust-loc: skipping unwalkable path");
                continue;
            }
        };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }

        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                // Debug-format the path so embedded newlines or ANSI
                // escapes cannot forge log lines, matching the
                // project-wide path-log policy.
                tracing::warn!(path = ?path, %error, "rust-loc: skipping unreadable file");
                continue;
            }
        };

        let relative = relativize_path(path, working_dir);
        let counts = count_source(&source, region_from_path(Path::new(&relative)));
        push_records(&mut records, &relative, &counts);
    }

    Ok(serde_json::Value::Array(records))
}

/// Is this entry a build/VCS directory to prune?
///
/// The walk root (depth 0) is never pruned: a workspace that happens to
/// be called `target` is still the directory the operator asked for.
fn is_excluded_dir(entry: &DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_some_and(|ft| ft.is_dir()) {
        return false;
    }
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| EXCLUDED_DIRS.contains(&name))
}

fn push_records(records: &mut Vec<serde_json::Value>, file: &str, counts: &FileCounts) {
    for (region, locs) in counts.non_empty() {
        records.push(serde_json::json!({
            "file": file,
            "region": region.as_str(),
            "code": locs.code,
            "docs": locs.docs,
            "comments": locs.comments,
            "blanks": locs.blanks,
            "lines": locs.lines(),
        }));
    }
}

/// Render a path as a workspace-relative UTF-8 string.
///
/// Intentionally lossy, for the same reason documented on the `tokei`
/// extension's `relativize_path`: the `rust_loc_files` column is
/// read-only at the value level and is populated from a JSON sidecar
/// rather than interpolated into SQL, so a `U+FFFD` substitution
/// affects display and prefix-join attribution only.
fn relativize_path(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn query_rust_loc_files(db: &DuckDb) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::query_rows_to_json(
        db,
        "SELECT file, region, code, docs, comments, blanks, lines FROM rust_loc_files",
        |row| {
            Ok(serde_json::json!({
                "file": row.get::<_, String>(0)?,
                "region": row.get::<_, String>(1)?,
                "code": row.get::<_, i64>(2)?,
                "docs": row.get::<_, i64>(3)?,
                "comments": row.get::<_, i64>(4)?,
                "blanks": row.get::<_, i64>(5)?,
                "lines": row.get::<_, i64>(6)?,
            }))
        },
    )
}

fn provide_from_db(db: &DuckDb, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::provide_via_ingestor(
        db,
        ctx,
        "rust_loc_files",
        &RustLocIngestor,
        query_rust_loc_files,
    )
}
