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
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, Deadline,
    ExtensionType,
};
use std::path::Path;
use tokei::{Config as TokeiConfig, LanguageType, Languages};

/// Extension identifier used to register this crate in the engine's
/// extension registry.
pub const NAME: &str = "tokei";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "Code statistics provider (lines of code, comments, blanks)";
/// CLI-facing short name (`tokei`) used in commands and user-facing output.
pub const SHORTNAME: &str = "tokei";
/// Registry key of the `tokei` data provider this crate registers —
/// the key the about code/loc subpages look the statistics up by.
pub const DATA_PROVIDER_NAME: &str = "tokei";

/// Datasource extension exposing tokei-derived per-file code statistics
/// under the [`DATA_PROVIDER_NAME`] key.
pub struct TokeiExtension;

ops_extension::impl_extension! {
    TokeiExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(TokeiProvider));
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
            collect_tokei(ctx.working_directory(), ctx.deadline_handle().as_ref())
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
        // CL-3 / TASK-2153: explicit ORDER BY so the queried path is ordered
        // too, not only the ingested one — DuckDB makes no row-order promise
        // for an unordered SELECT.
        "SELECT language, file, code, comments, blanks, lines FROM tokei_files ORDER BY file, language",
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
    /// Upper bound on the number of files **scanned** — that is, opened,
    /// counted, and materialised as records.
    ///
    /// Counts candidates, not directory entries. A file tokei has no language
    /// for is never opened, so it does not consume this budget; capping on
    /// every regular file instead would make an asset-heavy but entirely
    /// ordinary repository truncate before reaching any source, trading a
    /// correct statistic for a warned-but-wrong one. This bounds the
    /// expensive half of the walk (read + count), not the cheap half
    /// (`read_dir` + an extension lookup), which `depth` bounds instead.
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
///
/// Also `DataProviderError::TimedOut`, boxed into `anyhow`, if `deadline` is
/// supplied and expires mid-walk. That one *is* fatal: the scan is abandoned
/// rather than reported as a short count, because a truncated statistic
/// indistinguishable from a real one is worse than no statistic.
pub(crate) fn scan_tokei(
    working_dir: &Path,
    limits: ScanLimits,
    deadline: Option<&Deadline>,
) -> anyhow::Result<TokeiScan> {
    validate_scan_root(working_dir)?;
    let (candidates, mut skips) = collect_candidates(working_dir, limits, deadline)?;

    // Nothing left to count: skip the dispatch loop entirely and pass the
    // skip accounting through untouched.
    if candidates.is_empty() {
        return Ok(skips.into_empty_scan());
    }

    // PERF-3 / TASK-2159: count the candidates directly with
    // `LanguageType::parse` instead of handing them back to
    // `Languages::get_statistics`. Tokei's `get_statistics` does not treat
    // its slice as a file list — `utils::fs::get_all_files` (tokei 14.0.0)
    // builds a second `WalkBuilder` with one root per candidate and re-runs
    // the whole `ignore` pipeline on each (gitignore resolution, hidden
    // rules, a fresh `stat`) plus a fresh `LanguageType::from_path`: the
    // walk and classification `collect_candidates` already performed.
    // Parsing each already-filtered candidate once removes the second walk
    // and reports per-file open errors directly instead of inferring them
    // from a records-vs-candidates shortfall.
    let config = TokeiConfig::default();
    let mut languages = Languages::new();
    for path in candidates {
        // SEC-33 / TASK-2052: the parse loop now owns the file opens too, so
        // the cooperative cancellation point covers it — an open() on a
        // wedged mount blocks exactly like the `read_dir` half of the walk.
        if let Some(deadline) = deadline {
            deadline.check()?;
        }
        // Re-classify rather than trusting the walk's verdict: an extension
        // lookup is cheap, and a file replaced between the walk and the open
        // would otherwise be parsed under a stale language.
        let Some(lang) = LanguageType::from_path(&path, &config) else {
            continue;
        };
        match lang.parse(path, &config) {
            Ok(report) => {
                // Same accumulation tokei's own pipeline performs: group
                // per-language, one `Report` per file.
                languages.entry(lang).or_default().add_report(report);
            }
            Err((error, path)) => {
                skips.unreadable = skips.unreadable.saturating_add(1);
                // Debug-format the path per the project-wide path-log policy.
                tracing::warn!(path = ?path, %error, "tokei: candidate could not be opened");
            }
        }
    }
    let records = flatten_tokei_records(&languages, working_dir);

    Ok(skips.into_scan(records))
}

/// What the candidate walk refused to look at, in one value — FN-1 /
/// TASK-2161: the accounting is stated once here instead of spread across
/// three separately mutated locals in the walk loop.
#[derive(Debug, Default)]
struct Skips {
    /// Files skipped for exceeding [`ScanLimits::file_bytes`].
    oversize: usize,
    /// Files or subtrees that could not be read: a walk error, unreadable
    /// metadata, or a candidate the parse loop (TASK-2159) failed to open.
    unreadable: usize,
    /// Whether [`ScanLimits::files`] cut the walk short. When true the
    /// records are a prefix of the truth, not the whole of it.
    truncated: bool,
}

impl Skips {
    /// The scan outcome for an empty candidate set — counting is skipped
    /// entirely, so the skip counts pass through untouched.
    const fn into_empty_scan(self) -> TokeiScan {
        TokeiScan {
            records: Vec::new(),
            skipped_oversize: self.oversize,
            skipped_unreadable: self.unreadable,
            truncated: self.truncated,
        }
    }

    /// The scan outcome once tokei has produced `records`.
    const fn into_scan(self, records: Vec<serde_json::Value>) -> TokeiScan {
        TokeiScan {
            records,
            skipped_oversize: self.oversize,
            skipped_unreadable: self.unreadable,
            truncated: self.truncated,
        }
    }
}

/// Validate the scan root exists and is a directory.
///
/// # Errors
///
/// If `working_dir` does not exist, cannot be stat'd, or is not a directory.
fn validate_scan_root(working_dir: &Path) -> anyhow::Result<()> {
    let metadata = std::fs::metadata(working_dir)
        .with_context(|| format!("tokei: cannot read scan root {}", working_dir.display()))?;
    anyhow::ensure!(
        metadata.is_dir(),
        "tokei: scan root {} is not a directory",
        working_dir.display()
    );
    Ok(())
}

/// Walk `working_dir` and collect the paths tokei should count, applying the
/// four skip policies (walk error, non-file, unrecognised language,
/// unreadable metadata) and the two bound checks (`file_bytes`, `files`).
///
/// Failures *below* the root are not errors: an unreadable file or subtree is
/// counted in [`Skips::unreadable`] and the walk continues, since a partial
/// count with a warning beats no count at all.
///
/// # Errors
///
/// `DataProviderError::TimedOut`, boxed into `anyhow`, if `deadline` is
/// supplied and expires mid-walk. That one *is* fatal: the scan is abandoned
/// rather than reported as a short count, because a truncated statistic
/// indistinguishable from a real one is worse than no statistic.
fn collect_candidates(
    working_dir: &Path,
    limits: ScanLimits,
    deadline: Option<&Deadline>,
) -> anyhow::Result<(Vec<std::path::PathBuf>, Skips)> {
    let config = TokeiConfig::default();
    let mut candidates = Vec::new();
    let mut skips = Skips::default();

    let walker = WalkBuilder::new(working_dir)
        .max_depth(Some(limits.depth))
        .filter_entry(|entry| !is_pruned_dir(entry))
        .build();

    for entry in walker {
        // SEC-33 / TASK-2052: the cooperative cancellation point. `ScanLimits`
        // caps how much of a tree is walked, but a cap is not a clock: 50k
        // entries on a wedged network mount can outlast any budget, and the
        // dispatch bound would otherwise only *report* that after the fact.
        // Checked per directory entry rather than per counted file, because
        // the cheap-looking half of the walk (`read_dir`, a `stat`) is exactly
        // the half that blocks on such a mount.
        if let Some(deadline) = deadline {
            deadline.check()?;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                skips.unreadable = skips.unreadable.saturating_add(1);
                tracing::warn!(%error, "tokei: skipping unwalkable path");
                continue;
            }
        };
        let Some(path) = screen_entry(&entry, &config, limits, &mut skips) else {
            continue;
        };
        if candidates.len() >= limits.files {
            skips.truncated = true;
            tracing::warn!(
                cap = limits.files,
                "tokei: file cap reached; statistics are truncated"
            );
            break;
        }
        candidates.push(path);
    }

    Ok((candidates, skips))
}

/// Apply the per-entry skip policies and return the path to count.
///
/// An entry is refused three ways: non-file and unrecognised-language entries
/// are neither candidates nor skips (nothing is warned about), while
/// unreadable metadata and an over-[`ScanLimits::file_bytes`] size are
/// counted in [`Skips`] with a `tracing::warn!` each. Returns `Some(path)`
/// when the entry is a candidate the caller should account against
/// [`ScanLimits::files`].
fn screen_entry(
    entry: &DirEntry,
    config: &TokeiConfig,
    limits: ScanLimits,
    skips: &mut Skips,
) -> Option<std::path::PathBuf> {
    if !entry.file_type().is_some_and(|ft| ft.is_file()) {
        return None;
    }
    let path = entry.path();
    // Classify before stat'ing nothing else: a file tokei has no language
    // for is not scanned, so it is neither a candidate nor a skip.
    let _language = LanguageType::from_path(path, config)?;
    let file_len = match entry.metadata() {
        Ok(metadata) => metadata.len(),
        Err(error) => {
            skips.unreadable = skips.unreadable.saturating_add(1);
            // Debug-format the path so embedded newlines or ANSI escapes
            // cannot forge log lines, per the project-wide path-log policy.
            tracing::warn!(path = ?path, %error, "tokei: skipping file with unreadable metadata");
            return None;
        }
    };
    if file_len > limits.file_bytes {
        skips.oversize = skips.oversize.saturating_add(1);
        tracing::warn!(
            path = ?path,
            bytes = file_len,
            cap = limits.file_bytes,
            "tokei: skipping oversized file"
        );
        return None;
    }
    Some(path.to_path_buf())
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
/// If `working_dir` does not exist, cannot be stat'd, or is not a directory,
/// or if `deadline` expires mid-walk. A directory that genuinely holds no
/// recognised source returns `Ok([])`, which is therefore distinguishable
/// from an unreadable root.
pub fn collect_tokei(
    working_dir: &Path,
    deadline: Option<&Deadline>,
) -> Result<serde_json::Value, anyhow::Error> {
    let scan = scan_tokei(working_dir, ScanLimits::DEFAULT, deadline)?;
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

/// Flatten tokei's per-language report tree into one JSON record per file,
/// sorted by [`row_key`] — file path with language as tiebreak.
///
/// The public `flatten_tokei_to_json` wrapper that used to sit in front of
/// this was left with no production caller once `collect_tokei` started
/// counting skipped files (ERR-2, TASK-1972), so it went with the change
/// rather than staying as unreferenced public surface.
///
/// CL-3 / TASK-2153: tokei fills each language's `reports` in arbitrary
/// order — its `get_all_files` drives a crossbeam channel through
/// `par_bridge()` and `add_report`s from whichever rayon worker finishes
/// first (tokei 14.0.0, `src/utils/fs.rs`) — so the stored order is
/// worker-scheduling dependent and differed run to run. Sorting here, with
/// the same policy as `extensions-rust/loc`'s `row_key`, keeps the JSON
/// sidecar and the `DuckDB` ingest byte-stable across runs: a diff of two
/// collections shows real changes only, and `data_sources.checksum` stays a
/// useful change signal instead of churning on scheduler noise.
pub(crate) fn flatten_tokei_records(
    languages: &Languages,
    workspace_root: &Path,
) -> Vec<serde_json::Value> {
    let mut records: Vec<serde_json::Value> = languages
        .iter()
        .flat_map(|(lang_type, language)| {
            language
                .reports
                .iter()
                .map(move |report| report_to_json(lang_type.name(), report, workspace_root))
        })
        .collect();
    records.sort_by(|a, b| row_key(a).cmp(&row_key(b)));
    records
}

/// Sort key giving the emitted records a deterministic order: file path,
/// with language as tiebreak, matching the `row_key` policy in
/// `extensions-rust/loc` (CL-3 / TASK-2153).
fn row_key(record: &serde_json::Value) -> (&str, &str) {
    (
        record["file"].as_str().unwrap_or_default(),
        record["language"].as_str().unwrap_or_default(),
    )
}

fn report_to_json(
    language: &str,
    report: &tokei::Report,
    workspace_root: &Path,
) -> serde_json::Value {
    // DUP-1 / TASK-2183: the shared sidecar-path policy lives in
    // `ops_duckdb::sql::relativize_path`, with the lossy-conversion
    // rationale documented on it once.
    let file_str = ops_duckdb::sql::relativize_path(&report.name, workspace_root);
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
