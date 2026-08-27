//! llvm-cov JSON parsing and flattening.
//!
//! ARCH-1 / TASK-1559: lifted out of `lib.rs` so the wiring layer stays
//! focused. DUP-3 / TASK-1555: the per-file row schema is owned by
//! [`CoverageRow`] — the schema field list, the flatten output, the
//! `query_coverage_files` projection, and the in-crate test fixtures all
//! resolve to this one struct.

use crate::subprocess::{check_llvm_cov_output, format_cargo_exit, run_cargo_llvm_cov};
use anyhow::Context as AnyhowContext;
use ops_core::output::format_error_tail;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// DUP-3 / TASK-1555: single source of truth for the 15-field per-file
/// coverage row. The provider schema, flatten output, `query_coverage_files`
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

/// TASK-1599: batches schema-drift warnings so N malformed files produce at
/// most one warn per (section, field) pair per `flatten_coverage_json` call.
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

/// ERR-1 / TASK-1599: an absent field is legitimately empty (default); a
/// field that is `null` is downgraded to `debug!` (harmless absent marker);
/// a field that is present but the wrong shape (e.g. llvm-cov bumping `count`
/// to a string) is a schema-drift signal surfaced via [`DriftTracker`].
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

/// FN-1 / TASK-1553: build a single `CoverageRow` for one entry in
/// `files[]`. Returns `None` when `filename` is absent or non-string so the
/// caller can skip the record (TASK-0984: empty-key rows used to inflate
/// project totals).
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
                "TASK-0984: coverage file record has missing or non-string filename; skipping (llvm-cov schema drift?)"
            );
            return None;
        }
    };
    // `serde_json::Value::Null` and an empty object yield identical lookups
    // through `get(...)`, so we elide the `json!({})` allocation entirely.
    let summary = file.get("summary").unwrap_or(&serde_json::Value::Null);
    Some(CoverageRow::from_summary(filename, summary, drift_warned))
}

/// FN-1 / TASK-1553 + PATTERN-3 / TASK-1558: push `record` onto `records`,
/// or overwrite the prior slot when its filename was already seen.
///
/// PERF-3 / TASK-1598: uses `get` + conditional `insert` so the duplicate
/// (Occupied) path avoids the filename clone that `entry()` would require.
/// Only first-seen filenames incur one clone for the `HashMap` key.
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

/// FN-1 / TASK-1553: extracted from the previous 106-line monolith. Reads
/// as: validate top-level shape → for each export build records → dedup →
/// optionally warn. The per-record construction lives in [`build_record`],
/// the dedup branch in [`dedup_push`].
#[must_use = "flatten output drives coverage_files ingest; dropping it loses every per-file row"]
pub fn flatten_coverage_json(raw: &serde_json::Value) -> Result<serde_json::Value, anyhow::Error> {
    let data = raw
        .get("data")
        .and_then(|d| d.as_array())
        .context("missing or invalid 'data' array in coverage JSON")?;
    if data.is_empty() {
        anyhow::bail!("'data' array is empty in coverage JSON");
    }
    // ERR-1: cargo llvm-cov --json's `data` is an array (one entry per
    // export); future per-target merging produces multiple exports. Iterate
    // every entry instead of silently dropping data[1..].
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
    let total: usize = file_arrays.iter().map(|f| f.len()).sum();
    let mut records: Vec<CoverageRow> = Vec::with_capacity(total);
    // ERR-1 / TASK-1021: dedup by filename across all data[] exports.
    // Last-write-wins keeps `coverage_summary` SUM aggregates honest when a
    // future per-target merge surfaces the same filename in two exports.
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
            "TASK-1021: coverage JSON contained duplicate filename rows across data[] exports; \
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

/// Run `cargo llvm-cov` and flatten its JSON output into per-file records.
///
/// ERR-1 / TASK-1057: with `--no-fail-fast`, `cargo llvm-cov` still exits
/// non-zero when one or more tests fail, but the report file contains a
/// complete llvm-cov JSON document for the passing slice of the workspace. Treat
/// that case as a soft failure: warn (so the operator still sees the test
/// breakage in the log) and continue with the partial-but-useful coverage
/// data instead of dropping every per-file row.
///
/// ERR-1 / TASK-1557: the soft-fail predicate requires a **non-empty**
/// `data` array. An empty `data` array means cargo failed before
/// instrumenting anything; surfacing the original `check_llvm_cov_output`
/// error (with the cargo exit code + stderr tail) keeps the operator
/// pointed at the real root cause instead of the misleading "data array
/// is empty" message from `flatten_coverage_json`.
///
/// On the success path, non-empty stderr is emitted at `info` level so
/// instrumentation skips and compiler warnings are visible in operator
/// logs without re-running with `RUST_LOG=debug`.
#[must_use = "collect_coverage drives the coverage ingest; dropping the result throws the run away"]
pub fn collect_coverage(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error> {
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
    let output = run_cargo_llvm_cov(working_dir, &report_path)?;
    if !output.status.success() {
        let parsed = std::fs::read(report.path())
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        // READ-5 / TASK-1609: the predicate and value recovery are unified in
        // one `if let` so the compiler enforces the "parsed is Some" invariant
        // instead of a runtime `expect`.
        if let Some(valid_parsed) = parsed.as_ref().filter(|raw| {
            raw.get("data").and_then(|d| d.as_array()).is_some_and(|a| {
                !a.is_empty()
                    && a.iter()
                        .all(|e| e.get("files").and_then(|f| f.as_array()).is_some())
            })
        }) {
            let tail = format_error_tail(&output.stderr, 5);
            let marker = format_cargo_exit(output.status);
            tracing::warn!(
                exit = %marker,
                stderr_tail = %tail,
                "TASK-1057: cargo llvm-cov exited non-zero but stdout contains parseable JSON; \
                 continuing with partial coverage data (likely test failures with --no-fail-fast)"
            );
            return flatten_coverage_json(valid_parsed);
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
    let bytes = std::fs::read(report.path()).context("reading llvm-cov JSON report")?;
    let raw: serde_json::Value =
        serde_json::from_slice(&bytes).context("parsing llvm-cov JSON output")?;
    flatten_coverage_json(&raw)
}
