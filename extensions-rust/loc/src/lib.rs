//! Rust LOC extension: production / test / example line counts with doc
//! comments split from ordinary comments.
//!
//! Complements the language-agnostic `tokei` extension rather than
//! replacing it. `tokei_files` remains the source of truth for
//! cross-language totals; this crate adds the Rust-only breakdown that
//! `tokei` has no model for, since `#[cfg(test)]` blocks live in the
//! same file as the code they exercise.

// `src/tests.rs` relies on `unwrap()`; the crate performs no numeric
// casts, in tests or otherwise, so no cast lint is suppressed here.
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod counter;
mod ingestor;
#[cfg(test)]
mod tests;
pub mod views;

pub use ingestor::RustLocIngestor;

use std::io::{BufRead as _, BufReader};
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use ignore::{DirEntry, WalkBuilder, WalkState};
use ops_duckdb::DuckDb;
use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};

use counter::{count_source, region_from_path, FileCounts, Region};

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

/// Largest `.rs` file that is read into memory, lexed, and parsed.
///
/// `count_source` holds several times a file's byte size resident while
/// it runs: the owned source, one slice and one `LineKind` per line, the
/// full `proc_macro2::TokenStream`, the `syn::File` AST, and
/// proc-macro2's `span-locations` source map (which retains its own copy
/// of the source plus a line table). Nothing in a workspace is off
/// limits to the walk — vendored bindgen output, a generated parser
/// table, a concatenated build artifact — so without a gate one
/// machine-written file can OOM the whole `ops` process.
///
/// Over-cap files are still counted, by
/// [`counter::FileCounts::add_fallback_line`] over a streaming read, so
/// the degradation costs test attribution rather than the file itself.
/// First-party Rust source does not approach this size.
///
/// The cap is **per file, not per scan**: the walk runs on `ignore`'s
/// parallel walker, so peak resident cost is this figure multiplied by the
/// worker count (`ignore` defaults to the available parallelism). The cap
/// bounds what any one worker can hold, not the process total.
pub const MAX_SOURCE_BYTES: u64 = 4 * 1024 * 1024;

/// Walk `working_dir` for `.rs` files and classify each one.
///
/// Anything that cannot be read — an individual file, or a whole
/// subtree the walker cannot descend — is logged and skipped rather
/// than aborting the scan: a partial count is more useful than none,
/// and an unreadable path is not a data-integrity problem for a
/// display-only statistic. A file larger than [`MAX_SOURCE_BYTES`]
/// degrades the same way, to a streaming blank-vs-non-blank count
/// instead of a parse. Every one of those paths warns, so a silently
/// short or degraded count is always accompanied by a log line.
///
/// The walk runs on `ignore`'s parallel walker; rows are sorted by
/// `(file, region)` before returning, so the output does not depend on
/// worker scheduling.
///
/// # Errors
///
/// None today: every failure mode above is warned and skipped, per the
/// degradation policy just described, and the only exit builds a
/// `serde_json::Value::Array` from an already-materialised `Vec`, which
/// cannot fail. The `Result` is kept because
/// `ops_duckdb::try_provide_from_db` and `DataIngestor::collect` both
/// require a fallible closure, and because a future cancellation check
/// or hard resource limit would legitimately use it. Do not convert the
/// warn-and-skip branches into `?` to make this section true — the
/// partial-count policy is deliberate.
pub fn collect_rust_loc(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error> {
    // Counting is CPU-bound, per-file independent, and shares no mutable
    // state: proc-macro2's `span-locations` source map is a thread-local
    // and `invalidate_current_thread_spans` only touches the calling
    // thread's copy, so each worker simply keeps its own. The only shared
    // state is the row sink, locked once per file.
    let records = Mutex::new(Vec::new());

    WalkBuilder::new(working_dir)
        .filter_entry(|entry| !is_excluded_dir(entry))
        .build_parallel()
        .run(|| {
            Box::new(|entry| {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        tracing::warn!(%error, "rust-loc: skipping unwalkable path");
                        return WalkState::Continue;
                    }
                };
                if let Some((relative, counts)) = count_entry(&entry, working_dir) {
                    let mut sink = records.lock().unwrap_or_else(PoisonError::into_inner);
                    push_records(&mut sink, &relative, &counts);
                }
                WalkState::Continue
            })
        });

    let mut records = records.into_inner().unwrap_or_else(PoisonError::into_inner);
    // Workers finish in arbitrary order. Sorting keeps the JSON sidecar
    // and the DuckDB ingest byte-stable across runs, so a diff of two
    // collections shows real changes only.
    records.sort_by(|a, b| row_key(a).cmp(&row_key(b)));
    Ok(serde_json::Value::Array(records))
}

/// Sort key giving the emitted rows a deterministic order.
fn row_key(row: &serde_json::Value) -> (&str, &str) {
    (
        row["file"].as_str().unwrap_or_default(),
        row["region"].as_str().unwrap_or_default(),
    )
}

/// Classify one walk entry, or `None` if it is not a `.rs` file or
/// could not be read.
///
/// Every skip warns, per the degradation policy on [`collect_rust_loc`].
fn count_entry(entry: &DirEntry, working_dir: &Path) -> Option<(String, FileCounts)> {
    if !entry.file_type().is_some_and(|ft| ft.is_file()) {
        return None;
    }
    let path = entry.path();
    if path.extension().is_none_or(|ext| ext != "rs") {
        return None;
    }

    let relative = relativize_path(path, working_dir);
    let region = region_from_path(Path::new(&relative));

    // Gate on the walker's own metadata, before the contents are pulled
    // into memory: reading first and measuring afterwards would already
    // have paid the allocation the cap exists to avoid. Debug-format
    // every path so embedded newlines or ANSI escapes cannot forge log
    // lines, matching the project-wide path-log policy.
    let size = match entry.metadata() {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            tracing::warn!(path = ?path, %error, "rust-loc: skipping file with unreadable metadata");
            return None;
        }
    };

    let counts = if size > MAX_SOURCE_BYTES {
        tracing::warn!(
            path = ?path,
            size,
            max_bytes = MAX_SOURCE_BYTES,
            "rust-loc: file over the size cap; counting blank vs non-blank only"
        );
        match count_streaming(path, region) {
            Ok(counts) => counts,
            Err(error) => {
                tracing::warn!(path = ?path, %error, "rust-loc: skipping unreadable file");
                return None;
            }
        }
    } else {
        match std::fs::read_to_string(path) {
            // `count_source` warns on the nesting-depth cap but takes only
            // `&str`, so it has no path to name. Entering a span here adds the
            // field to that warn without widening its signature; every other
            // warn on this path already Debug-formats the path itself.
            Ok(source) => {
                let _span = tracing::warn_span!("rust-loc.count_source", path = ?path).entered();
                count_source(&source, region)
            }
            Err(error) => {
                tracing::warn!(path = ?path, %error, "rust-loc: skipping unreadable file");
                return None;
            }
        }
    };

    Some((relative, counts))
}

/// Count an over-cap file without holding it in memory.
///
/// The blank-vs-non-blank split of [`counter::count_fallback`], fed one
/// line at a time. Reads bytes rather than `str` so that invalid UTF-8
/// in a machine-written file still yields a count, and treats an
/// all-ASCII-whitespace line as blank.
fn count_streaming(path: &Path, region: Region) -> std::io::Result<FileCounts> {
    let mut reader = BufReader::new(std::fs::File::open(path)?);
    let mut counts = FileCounts::default();
    // Blank-vs-non-blank state for the line currently being scanned, carried
    // across chunk boundaries. `started` marks bytes seen since the last
    // newline, so a trailing newline does not manufacture an extra line.
    let mut blank = true;
    let mut started = false;

    // Scan `BufReader`'s own buffer in place. Nothing beyond that fixed
    // capacity is ever held, which is the whole point of this path.
    loop {
        let consumed = {
            let chunk = reader.fill_buf()?;
            if chunk.is_empty() {
                break;
            }
            for &byte in chunk {
                if byte == b'\n' {
                    counts.add_fallback_line(region, blank);
                    blank = true;
                    started = false;
                } else {
                    started = true;
                    if !byte.is_ascii_whitespace() {
                        blank = false;
                    }
                }
            }
            chunk.len()
        };
        reader.consume(consumed);
    }

    // A final line with no trailing newline still counts.
    if started {
        counts.add_fallback_line(region, blank);
    }

    Ok(counts)
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
