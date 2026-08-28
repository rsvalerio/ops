//! Tokei extension: code statistics (lines of code, comments, blanks) via the tokei library.
//! Language-agnostic -- loads for any project regardless of stack.
//!
//! `load_tokei` was removed in favour of the single [`TokeiIngestor`] entry
//! point (DUP-1, TASK-0226). That invariant is enforced here rather than
//! asserted in prose: this doctest stops compiling the day the symbol comes
//! back, which is exactly when it should fail (TEST-1, TASK-1978).
//!
//! ```compile_fail
//! let _ = ops_tokei::load_tokei;
//! ```

// READ-10 (TASK-1968): this crate root carries no `cfg_attr(test, allow(..))`
// block. All four lints it used to relax suppress nothing here. The three cast
// lints have no callsite -- the crate contains no `as` cast, and the workspace
// denies `clippy::as_conversions` anyway -- and `unwrap_used` is already
// relaxed for test code workspace-wide by `allow-unwrap-in-tests` in
// `clippy.toml`, so writing it as `expect` reports it as unfulfilled.

mod ingestor;
#[cfg(test)]
mod tests;
pub mod views;

pub use ingestor::TokeiIngestor;

use anyhow::Context as _;
use ignore::{DirEntry, WalkBuilder};
use ops_duckdb::DuckDb;
use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use std::path::Path;
use tokei::{Config as TokeiConfig, LanguageType, Languages};

pub const NAME: &str = "tokei";
pub const DESCRIPTION: &str = "Code statistics provider (lines of code, comments, blanks)";
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

/// Top-level directory names pruned from the scan.
///
/// CL-3 (TASK-1974): these are matched **by exact name, against direct
/// children of the scan root only** — see [`is_pruned_dir`]. They are not
/// gitignore globs. An earlier revision handed this list to tokei's own
/// walker, which turned each entry into an unanchored `!name` override: that
/// dropped a `build/` package nested under `src/`, and dropped plain *files*
/// named `dist` or `build`, neither of which the name suggests. The walk is
/// now ours, so the anchoring is ours too and the doc matches the code.
///
/// **Redundancy is deliberate.** Inside a git repository `.gitignore` already
/// hides most of these, and the walker honours it. The list is what keeps the
/// counts sane on an unversioned checkout, where no ignore file exists — the
/// same call the sibling `rust-loc` extension makes.
///
/// **Not project-overridable, deliberately.** `tokei_files` is an advisory
/// display statistic, and a project that keeps source under `target/` or
/// `build/` at its root still gets it counted by adding that path to its own
/// `.gitignore` negation rules, which the walker reads. Adding an `.ops.toml`
/// key would introduce a second, tokei-only exclusion dialect that reaches no
/// further than the existing one; revisit only for a project that cannot be
/// expressed that way.
pub(crate) const TOKEI_DEFAULT_EXCLUDED: &[&str] = &[
    "target",
    ".git",
    "node_modules",
    ".venv",
    "venv",
    "dist",
    "build",
];

/// Bounds on a single scan.
///
/// SEC-33 (TASK-1970): `working_dir` is whatever directory the operator points
/// `ops` at, and the tree under it is arbitrary third-party content. Every
/// dimension of the walk that could otherwise grow without bound is capped
/// here, so a hostile or merely unusual tree degrades the statistic instead of
/// taking the process down.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ScanLimits {
    /// Files larger than this are skipped without being opened. Tokei reads
    /// each file whole into memory to count its lines, so a checked-in
    /// multi-GB `.sql` dump would otherwise be resident all at once.
    pub file_bytes: u64,
    /// Upper bound on the number of files scanned, and therefore on the
    /// number of records materialised.
    pub files: usize,
    /// Upper bound on walk depth. `ignore` defaults to unlimited.
    pub depth: usize,
}

impl ScanLimits {
    /// Defaults sized for a source tree: no hand-written source file
    /// approaches 4 MiB, 50k files covers a large monorepo, and 32 levels is
    /// well past any real source layout.
    pub const DEFAULT: Self = Self {
        file_bytes: 4 * 1024 * 1024,
        files: 50_000,
        depth: 32,
    };
}

/// The outcome of one scan, including what it refused to look at.
///
/// ERR-2 (TASK-1972): the counts exist so a short answer is distinguishable
/// from a correct one. `collect_tokei` folds them into a warning; callers that
/// want them structurally use [`scan_tokei`].
#[derive(Debug)]
pub(crate) struct TokeiScan {
    pub records: Vec<serde_json::Value>,
    /// Files skipped because they exceeded [`ScanLimits::file_bytes`].
    pub skipped_oversize: usize,
    /// Files or subtrees that could not be read: a walk error, unreadable
    /// metadata, or a file tokei itself failed to open.
    pub skipped_unreadable: usize,
    /// Whether [`ScanLimits::files`] cut the walk short. When true the
    /// records are a prefix of the truth, not the whole of it.
    pub truncated: bool,
}

/// Walk `working_dir` and count every source file tokei recognises.
///
/// # Errors
///
/// If `working_dir` does not exist, cannot be stat'd, or is not a directory.
/// Failures *below* the root are not errors: an unreadable file or subtree is
/// counted in [`TokeiScan::skipped_unreadable`] and the scan continues, since
/// a partial count with a warning beats no count at all.
pub(crate) fn scan_tokei(working_dir: &Path, limits: ScanLimits) -> anyhow::Result<TokeiScan> {
    let metadata = std::fs::metadata(working_dir)
        .with_context(|| format!("tokei: cannot read scan root {}", working_dir.display()))?;
    anyhow::ensure!(
        metadata.is_dir(),
        "tokei: scan root {} is not a directory",
        working_dir.display()
    );

    let config = TokeiConfig::default();
    let mut candidates = Vec::new();
    let mut skipped_oversize = 0usize;
    let mut skipped_unreadable = 0usize;
    let mut truncated = false;

    let walker = WalkBuilder::new(working_dir)
        .max_depth(Some(limits.depth))
        .filter_entry(|entry| !is_pruned_dir(entry))
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                skipped_unreadable = skipped_unreadable.saturating_add(1);
                tracing::warn!(%error, "tokei: skipping unwalkable path");
                continue;
            }
        };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let path = entry.path();
        // Classify before stat'ing nothing else: a file tokei has no language
        // for is not scanned, so it is neither a candidate nor a skip.
        if LanguageType::from_path(path, &config).is_none() {
            continue;
        }
        let file_len = match entry.metadata() {
            Ok(metadata) => metadata.len(),
            Err(error) => {
                skipped_unreadable = skipped_unreadable.saturating_add(1);
                // Debug-format the path so embedded newlines or ANSI escapes
                // cannot forge log lines, per the project-wide path-log policy.
                tracing::warn!(path = ?path, %error, "tokei: skipping file with unreadable metadata");
                continue;
            }
        };
        if file_len > limits.file_bytes {
            skipped_oversize = skipped_oversize.saturating_add(1);
            tracing::warn!(
                path = ?path,
                bytes = file_len,
                cap = limits.file_bytes,
                "tokei: skipping oversized file"
            );
            continue;
        }
        if candidates.len() >= limits.files {
            truncated = true;
            tracing::warn!(
                cap = limits.files,
                "tokei: file cap reached; statistics are truncated"
            );
            break;
        }
        candidates.push(path.to_path_buf());
    }

    // `Languages::get_statistics` unwraps the first path, so an empty
    // candidate set must not reach it.
    if candidates.is_empty() {
        return Ok(TokeiScan {
            records: Vec::new(),
            skipped_oversize,
            skipped_unreadable,
            truncated,
        });
    }

    let mut languages = Languages::new();
    // The candidate list is already filtered, so tokei gets no exclusions:
    // every path handed to it is a file we decided to count.
    languages.get_statistics(&candidates, &[], &config);
    let records = flatten_tokei_records(&languages, working_dir);

    // Tokei drops any file it cannot open, with no counter of its own. Every
    // candidate was a recognised language, so the shortfall is exactly the set
    // of files it failed to read.
    skipped_unreadable =
        skipped_unreadable.saturating_add(candidates.len().saturating_sub(records.len()));

    Ok(TokeiScan {
        records,
        skipped_oversize,
        skipped_unreadable,
        truncated,
    })
}

/// Should this entry be pruned from the walk?
///
/// Only a directory that is a **direct child of the scan root** and whose name
/// is in [`TOKEI_DEFAULT_EXCLUDED`] is pruned. The root itself (depth 0) never
/// is: a workspace that happens to be named `build` is still the directory the
/// operator asked for.
fn is_pruned_dir(entry: &DirEntry) -> bool {
    if entry.depth() != 1 || !entry.file_type().is_some_and(|ft| ft.is_dir()) {
        return false;
    }
    entry
        .file_name()
        .to_str()
        .is_some_and(|name| TOKEI_DEFAULT_EXCLUDED.contains(&name))
}

/// Collect per-file statistics under `working_dir` as a JSON array.
///
/// Files that are oversized, unreadable, or past the scan's file cap are
/// skipped and reported through a warning; see [`ScanLimits`] and
/// [`TokeiScan`] for the exact accounting.
///
/// # Errors
///
/// If `working_dir` does not exist, cannot be stat'd, or is not a directory.
/// A directory that genuinely holds no recognised source returns `Ok([])`,
/// which is therefore distinguishable from an unreadable root.
pub fn collect_tokei(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error> {
    let scan = scan_tokei(working_dir, ScanLimits::DEFAULT)?;
    if scan.skipped_oversize > 0 || scan.skipped_unreadable > 0 || scan.truncated {
        tracing::warn!(
            skipped_oversize = scan.skipped_oversize,
            skipped_unreadable = scan.skipped_unreadable,
            truncated = scan.truncated,
            counted = scan.records.len(),
            "tokei: statistics are incomplete"
        );
    }
    Ok(serde_json::Value::Array(scan.records))
}

/// Flatten tokei's per-language report tree into one JSON record per file.
///
/// The public `flatten_tokei_to_json` wrapper that used to sit in front of
/// this was left with no production caller once `collect_tokei` started
/// counting skipped files (ERR-2, TASK-1972), so it went with the change
/// rather than staying as unreferenced public surface.
pub(crate) fn flatten_tokei_records(
    languages: &Languages,
    workspace_root: &Path,
) -> Vec<serde_json::Value> {
    languages
        .iter()
        .flat_map(|(lang_type, language)| {
            language
                .reports
                .iter()
                .map(move |report| report_to_json(lang_type.name(), report, workspace_root))
        })
        .collect()
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
/// READ-5 (TASK-0504): this is intentionally lossy. The `DuckDB` `tokei_files`
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
