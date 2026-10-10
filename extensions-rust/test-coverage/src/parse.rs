//! llvm-cov JSON parsing and flattening.
//!
//! The per-file row schema is owned by [`CoverageRow`] — the schema field
//! list, the flatten output, the `query_coverage_files` projection, and the
//! in-crate test fixtures all resolve to this one struct.

use crate::subprocess::{check_llvm_cov_output, format_cargo_exit, run_cargo_llvm_cov};
use anyhow::Context as AnyhowContext;
use ops_core::output::format_error_tail;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Single source of truth for the 15-field per-file coverage row. The
/// provider schema, flatten output, `query_coverage_files`
/// projection, and in-crate test fixtures all flow through this struct so
/// adding a new metric (e.g. `mcdc_*` if llvm-cov adds it) lights up the
/// compiler at every site instead of silently dropping the field somewhere.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageRow {
    pub(crate) filename: String,
    pub(crate) lines_count: i64,
    pub(crate) lines_covered: i64,
    pub(crate) lines_percent: f64,
    pub(crate) functions_count: i64,
    pub(crate) functions_covered: i64,
    pub(crate) functions_percent: f64,
    pub(crate) regions_count: i64,
    pub(crate) regions_covered: i64,
    pub(crate) regions_notcovered: i64,
    pub(crate) regions_percent: f64,
    pub(crate) branches_count: i64,
    pub(crate) branches_covered: i64,
    pub(crate) branches_notcovered: i64,
    pub(crate) branches_percent: f64,
}

impl CoverageRow {
    fn from_summary(
        filename: &str,
        summary: &serde_json::Value,
        drift_warned: &mut std::collections::HashSet<(String, String)>,
    ) -> Self {
        let lines = extract_section(summary, "lines", drift_warned);
        let functions = extract_section(summary, "functions", drift_warned);
        let regions = extract_section(summary, "regions", drift_warned);
        let branches = extract_section(summary, "branches", drift_warned);
        Self {
            filename: filename.to_string(),
            lines_count: lines.count,
            lines_covered: lines.covered,
            lines_percent: lines.percent,
            functions_count: functions.count,
            functions_covered: functions.covered,
            functions_percent: functions.percent,
            regions_count: regions.count,
            regions_covered: regions.covered,
            regions_notcovered: regions.notcovered,
            regions_percent: regions.percent,
            branches_count: branches.count,
            branches_covered: branches.covered,
            branches_notcovered: branches.notcovered,
            branches_percent: branches.percent,
        }
    }
}

/// Coverage section counters extracted from one of `lines` / `functions` /
/// `regions` / `branches` in the llvm-cov per-file `summary` block.
/// `notcovered` is only meaningful for region- and branch-level sections;
/// for lines and functions it is always zero.
#[derive(Default)]
struct Section {
    count: i64,
    covered: i64,
    notcovered: i64,
    percent: f64,
}

fn extract_section(
    summary: &serde_json::Value,
    key: &str,
    drift_warned: &mut std::collections::HashSet<(String, String)>,
) -> Section {
    let Some(s) = summary.get(key) else {
        return Section::default();
    };
    let mut drift = DriftTracker::new(key, drift_warned);
    Section {
        count: read_i64_field(s, "count", &mut drift),
        covered: read_i64_field(s, "covered", &mut drift),
        notcovered: read_i64_field(s, "notcovered", &mut drift),
        percent: read_f64_field(s, "percent", &mut drift),
    }
}

/// Batches schema-drift warnings so N malformed files produce at most one
/// warn per (section, field) pair per `flatten_coverage_json` call.
pub struct DriftTracker<'a> {
    section_key: &'a str,
    warned: &'a mut std::collections::HashSet<(String, String)>,
}

impl<'a> DriftTracker<'a> {
    pub(crate) const fn new(
        section_key: &'a str,
        warned: &'a mut std::collections::HashSet<(String, String)>,
    ) -> Self {
        Self {
            section_key,
            warned,
        }
    }
}

impl DriftTracker<'_> {
    pub(crate) fn warn_wrong_shape(
        &mut self,
        field: &str,
        value: &serde_json::Value,
        type_name: &'static str,
    ) {
        let key = (self.section_key.to_string(), field.to_string());
        if self.warned.insert(key) {
            tracing::warn!(
                section = self.section_key,
                field,
                value = %value,
                "coverage field present but not {type_name}; coercing to default (llvm-cov schema drift?)"
            );
        }
    }
}

/// An absent field is legitimately empty (default); a field that is `null`
/// is downgraded to `debug!` (harmless absent marker); a field that is
/// present but the wrong shape (e.g. llvm-cov bumping `count` to a string)
/// is a schema-drift signal surfaced via [`DriftTracker`].
fn read_field<T: Default>(
    section: &serde_json::Value,
    field: &str,
    accessor: impl FnOnce(&serde_json::Value) -> Option<T>,
    type_name: &'static str,
    drift: &mut DriftTracker<'_>,
) -> T {
    section.get(field).map_or_else(T::default, |v| {
        accessor(v).unwrap_or_else(|| {
            if v.is_null() {
                tracing::debug!(
                    section = drift.section_key,
                    field,
                    "coverage field is null; coercing to default"
                );
            } else {
                drift.warn_wrong_shape(field, v, type_name);
            }
            T::default()
        })
    })
}

fn read_i64_field(section: &serde_json::Value, field: &str, drift: &mut DriftTracker<'_>) -> i64 {
    read_field(
        section,
        field,
        serde_json::Value::as_i64,
        "an integer",
        drift,
    )
}

fn read_f64_field(section: &serde_json::Value, field: &str, drift: &mut DriftTracker<'_>) -> f64 {
    read_field(section, field, serde_json::Value::as_f64, "a float", drift)
}

/// Build a single `CoverageRow` for one entry in `files[]`. Returns `None`
/// when `filename` is absent, non-string, or empty so the caller can skip
/// the record — empty-key rows would otherwise inflate project totals.
fn build_record(
    file: &serde_json::Value,
    drift_warned: &mut std::collections::HashSet<(String, String)>,
) -> Option<CoverageRow> {
    let filename = match file.get("filename").and_then(|f| f.as_str()) {
        Some(s) if !s.is_empty() => s,
        other => {
            tracing::warn!(
                field = "filename",
                value = %other.map_or(serde_json::Value::Null, |s| serde_json::Value::String(s.to_string())),
                raw = %file.get("filename").unwrap_or(&serde_json::Value::Null),
                "coverage file record has missing or non-string filename; skipping (llvm-cov schema drift?)"
            );
            return None;
        }
    };
    // `serde_json::Value::Null` and an empty object yield identical lookups
    // through `get(...)`, so we elide the `json!({})` allocation entirely.
    let summary = file.get("summary").unwrap_or(&serde_json::Value::Null);
    Some(CoverageRow::from_summary(filename, summary, drift_warned))
}

/// Push `record` onto `records`, or overwrite the prior slot when its
/// filename was already seen.
///
/// Uses `get` + conditional `insert` so the duplicate (Occupied) path avoids
/// the filename clone that `entry()` would require. Only first-seen
/// filenames incur one clone for the `HashMap` key.
fn dedup_push(
    records: &mut Vec<CoverageRow>,
    idx_map: &mut std::collections::HashMap<String, usize>,
    record: CoverageRow,
    duplicate_count: &mut usize,
) {
    if let Some(&idx) = idx_map.get(&record.filename) {
        // `idx_map` only ever holds indices handed out by the `else` arm, so
        // a miss here would mean the two fell out of sync: keep the earlier
        // row instead of panicking the whole coverage ingest.
        if let Some(slot) = records.get_mut(idx) {
            *slot = record;
        }
        // At most one increment per element of the in-memory `files` arrays,
        // whose combined length is bounded by `isize::MAX`, so
        // `saturating_add` equals `+= 1` exactly.
        *duplicate_count = duplicate_count.saturating_add(1);
    } else {
        idx_map.insert(record.filename.clone(), records.len());
        records.push(record);
    }
}

/// Build the deduped per-file records from every export's `files` array.
///
/// Returns the records plus two accounting counts for the caller's summary
/// warns: records skipped because their filename was missing or non-string,
/// and duplicate filename rows overwritten by a later export.
fn build_records(file_arrays: Vec<&[serde_json::Value]>) -> (Vec<CoverageRow>, usize, usize) {
    let total: usize = file_arrays.iter().map(|f| f.len()).sum();
    let mut records: Vec<CoverageRow> = Vec::with_capacity(total);
    // Dedup by filename across all data[] exports: last-write-wins keeps
    // `coverage_summary` SUM aggregates honest when per-target merging
    // surfaces the same filename in two exports.
    let mut filename_to_idx: std::collections::HashMap<String, usize> =
        std::collections::HashMap::with_capacity(total);
    let mut duplicate_count: usize = 0;
    let mut skipped_count: usize = 0;
    let mut drift_warned: std::collections::HashSet<(String, String)> =
        std::collections::HashSet::new();
    for file in file_arrays.into_iter().flat_map(|f| f.iter()) {
        let Some(record) = build_record(file, &mut drift_warned) else {
            // At most one increment per element of the in-memory `files`
            // arrays, whose combined length is bounded by `isize::MAX`, so
            // `saturating_add` equals `+= 1` exactly.
            skipped_count = skipped_count.saturating_add(1);
            continue;
        };
        dedup_push(
            &mut records,
            &mut filename_to_idx,
            record,
            &mut duplicate_count,
        );
    }
    (records, skipped_count, duplicate_count)
}

/// Flattens the llvm-cov JSON document into per-file coverage rows. Reads
/// as: validate top-level shape → build deduped records → warn on skips and
/// duplicates. The per-record construction lives in [`build_record`], the
/// dedup branch in [`dedup_push`], and the records loop in [`build_records`].
#[must_use = "flatten output drives coverage_files ingest; dropping it loses every per-file row"]
pub fn flatten_coverage_json(raw: &serde_json::Value) -> Result<serde_json::Value, anyhow::Error> {
    let data = raw
        .get("data")
        .and_then(|d| d.as_array())
        .context("missing or invalid 'data' array in coverage JSON")?;
    if data.is_empty() {
        anyhow::bail!("'data' array is empty in coverage JSON");
    }
    // cargo llvm-cov --json's `data` is an array (one entry per export);
    // per-target merging produces multiple exports. Iterate every entry
    // instead of silently dropping data[1..].
    if data.len() > 1 {
        tracing::warn!(
            entries = data.len(),
            "coverage JSON contains more than one data export; flattening all entries"
        );
    }
    let file_arrays: Vec<&[serde_json::Value]> = data
        .iter()
        .map(|entry| {
            entry
                .get("files")
                .and_then(|f| f.as_array().map(std::vec::Vec::as_slice))
                .context("missing or invalid 'files' array in coverage data")
        })
        .collect::<Result<_, _>>()?;
    let (records, skipped_count, duplicate_count) = build_records(file_arrays);
    if skipped_count > 0 {
        tracing::warn!(
            skipped = skipped_count,
            valid_files = records.len(),
            "coverage JSON contained records with missing or non-string filenames; \
             skipped to keep coverage_summary aggregates honest"
        );
    }
    if duplicate_count > 0 {
        tracing::warn!(
            duplicates = duplicate_count,
            unique_files = records.len(),
            "coverage JSON contained duplicate filename rows across data[] exports; \
             applied last-write-wins dedup to keep coverage_summary aggregates honest"
        );
    }
    serde_json::to_value(records).context("encoding coverage rows as JSON")
}

/// Formats non-empty stderr as a diagnostic tail for logging. Returns
/// `None` when stderr is empty so the caller can skip the log line entirely.
pub fn format_stderr_diagnostic(stderr: &[u8]) -> Option<String> {
    if stderr.is_empty() {
        return None;
    }
    Some(format_error_tail(stderr, 5))
}

/// The soft-fail predicate, named once so [`collect_coverage_with`] and its
/// regression guard bind to the same code.
///
/// Returns `true` when the llvm-cov report recovered from a non-zero cargo
/// run is usable: `data` is an array, it is non-empty, and every entry
/// carries a `files` array.
///
/// An empty `data` array means cargo failed before instrumenting anything,
/// and an entry without `files` would surface a schema-shape parse error
/// instead of the cargo exit. Both must reject so the caller falls through
/// to the cargo error path.
pub fn has_parseable_coverage_data(raw: &serde_json::Value) -> bool {
    raw.get("data").and_then(|d| d.as_array()).is_some_and(|a| {
        !a.is_empty()
            && a.iter()
                .all(|e| e.get("files").and_then(|f| f.as_array()).is_some())
    })
}

/// Run `cargo llvm-cov` and flatten its JSON output into per-file records.
///
/// With `--no-fail-fast`, `cargo llvm-cov` still exits non-zero when one or
/// more tests fail, but the report file contains a complete llvm-cov JSON
/// document for the passing slice of the workspace. That case is treated as
/// a soft failure: warn (so the operator still sees the test breakage in
/// the log) and continue with the partial-but-useful coverage data instead
/// of dropping every per-file row.
///
/// The soft-fail predicate requires a **non-empty** `data` array. An empty
/// `data` array means cargo failed before instrumenting anything; surfacing
/// the original `check_llvm_cov_output` error (with the cargo exit code +
/// stderr tail) keeps the operator pointed at the real root cause instead
/// of the misleading "data array is empty" message from
/// `flatten_coverage_json`.
///
/// On the success path, non-empty stderr is emitted at `info` level so
/// instrumentation skips and compiler warnings are visible in operator
/// logs without re-running with `RUST_LOG=debug`.
///
/// `deadline` is the provider dispatch deadline (`Context::deadline`), which
/// sizes the `cargo llvm-cov` wait so the subprocess cannot outlive the
/// budget bounding it. `None` means unbounded dispatch and leaves the wait
/// at [`crate::subprocess::CARGO_LLVM_COV_TIMEOUT`].
/// Soft-fail recovery for a non-zero `cargo llvm-cov` exit.
///
/// Reads and parses the report file named by `report_path`. When it carries
/// usable coverage data ([`has_parseable_coverage_data`]), warns so the
/// operator still sees the test breakage and returns `Some` with the
/// flattened partial report — typically test failures under `--no-fail-fast`
/// that still leave a complete report for the passing slice of the workspace.
///
/// Returns `None` when no usable report exists so the caller falls through
/// to the cargo exit error: a report that cannot be read leaves a breadcrumb
/// warn naming the path and the IO error (the cargo exit stays the headline
/// error, but the failed read is a filesystem problem the operator would
/// otherwise never see), and a report without usable `data` means cargo
/// failed before instrumenting anything.
fn recover_partial_report(
    report_path: &Path,
    output: &std::process::Output,
) -> Option<Result<serde_json::Value, anyhow::Error>> {
    let parsed = match std::fs::read(report_path) {
        Ok(bytes) => serde_json::from_slice::<serde_json::Value>(&bytes).ok(),
        Err(err) => {
            tracing::warn!(
                report_path = %report_path.display(),
                error = %err,
                "could not read the llvm-cov JSON report after a non-zero cargo exit; \
                 falling through to the cargo error"
            );
            None
        }
    };
    let valid_parsed = parsed
        .as_ref()
        .filter(|raw| has_parseable_coverage_data(raw))?;
    let tail = format_error_tail(&output.stderr, 5);
    let marker = format_cargo_exit(output.status);
    tracing::warn!(
        exit = %marker,
        stderr_tail = %tail,
        report_path = %report_path.display(),
        "cargo llvm-cov exited non-zero but the JSON report file is parseable; \
         continuing with partial coverage data (likely test failures with --no-fail-fast)"
    );
    Some(flatten_coverage_json(valid_parsed))
}

#[must_use = "collect_coverage drives the coverage ingest; dropping the result throws the run away"]
pub fn collect_coverage(
    working_dir: &Path,
    deadline: Option<std::time::Instant>,
) -> Result<serde_json::Value, anyhow::Error> {
    collect_coverage_with(working_dir, |dir, output_path| {
        run_cargo_llvm_cov(dir, output_path, deadline)
    })
}

/// The body of [`collect_coverage`], with the cargo runner injected so the
/// soft-fail demotion, the hard-fail fall-through, and the report-read arms
/// are reachable from tests without running the whole workspace suite under
/// instrumentation.
///
/// `run` receives the working directory and the OS path of the report file
/// that `--output-path` names, so a test double can write a synthetic
/// report there exactly as cargo would.
#[must_use = "collect_coverage drives the coverage ingest; dropping the result throws the run away"]
pub fn collect_coverage_with<R>(
    working_dir: &Path,
    run: R,
) -> Result<serde_json::Value, anyhow::Error>
where
    R: FnOnce(&Path, &str) -> Result<std::process::Output, ops_core::subprocess::RunError>,
{
    // The JSON report is written to a temp file via `--output-path` rather
    // than captured from stdout: the report grows with the workspace and a
    // ~8 MB document blows past the OPS_OUTPUT_BYTE_CAP stdout cap, which
    // silently truncates it into unparseable JSON (and an opaque
    // "ingestor collect" failure). The handle keeps the file alive until
    // this function returns; the OS path is what cargo writes through.
    let report = tempfile::Builder::new()
        .prefix("ops-llvm-cov-")
        .suffix(".json")
        .tempfile()
        .context("creating temp file for llvm-cov JSON report")?;
    let report_path = report
        .path()
        .to_str()
        .context("llvm-cov temp report path is not valid UTF-8")?
        .to_string();
    let output = run(working_dir, &report_path)?;
    if !output.status.success() {
        if let Some(partial) = recover_partial_report(report.path(), &output) {
            return partial;
        }
        check_llvm_cov_output(&output)?;
    }
    if let Some(tail) = format_stderr_diagnostic(&output.stderr) {
        tracing::info!(
            stderr_tail = %tail,
            "cargo llvm-cov succeeded with stderr output; check for warnings or instrumentation skips"
        );
    } else {
        tracing::debug!("cargo llvm-cov completed successfully");
    }
    // Name the report file in both error contexts — the path is a tempfile
    // under TMPDIR, and it is what tells the operator
    // whether TMPDIR is full, read-only, or swept by a cleaner mid-run.
    let bytes = std::fs::read(report.path())
        .with_context(|| format!("reading llvm-cov JSON report {}", report.path().display()))?;
    let raw: serde_json::Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing llvm-cov JSON report {}", report.path().display()))?;
    flatten_coverage_json(&raw)
}
