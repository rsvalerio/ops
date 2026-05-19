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
pub(crate) struct CoverageRow {
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
    fn from_summary(filename: &str, summary: &serde_json::Value) -> Self {
        let lines = extract_section(summary, "lines");
        let functions = extract_section(summary, "functions");
        let regions = extract_section(summary, "regions");
        let branches = extract_section(summary, "branches");
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

fn extract_section(summary: &serde_json::Value, key: &str) -> Section {
    let Some(s) = summary.get(key) else {
        return Section::default();
    };
    Section {
        count: read_i64_field(s, key, "count"),
        covered: read_i64_field(s, key, "covered"),
        notcovered: read_i64_field(s, key, "notcovered"),
        percent: read_f64_field(s, key, "percent"),
    }
}

/// ERR-1: an absent field is legitimately empty (default); a field that is
/// present but the wrong shape (e.g. llvm-cov bumping `count` to a string or
/// float) is a schema-drift signal that must surface as a warn so coverage
/// does not silently drop to zero.
fn read_field<T: Default>(
    section: &serde_json::Value,
    section_key: &str,
    field: &str,
    accessor: impl FnOnce(&serde_json::Value) -> Option<T>,
    type_name: &'static str,
) -> T {
    match section.get(field) {
        None => T::default(),
        Some(v) => accessor(v).unwrap_or_else(|| {
            tracing::warn!(
                section = section_key,
                field,
                value = %v,
                "coverage field present but not {type_name}; coercing to default (llvm-cov schema drift?)"
            );
            T::default()
        }),
    }
}

fn read_i64_field(section: &serde_json::Value, section_key: &str, field: &str) -> i64 {
    read_field(
        section,
        section_key,
        field,
        serde_json::Value::as_i64,
        "an integer",
    )
}

fn read_f64_field(section: &serde_json::Value, section_key: &str, field: &str) -> f64 {
    read_field(
        section,
        section_key,
        field,
        serde_json::Value::as_f64,
        "a float",
    )
}

/// FN-1 / TASK-1553: build a single `CoverageRow` for one entry in
/// `files[]`. Returns `None` when `filename` is absent or non-string so the
/// caller can skip the record (TASK-0984: empty-key rows used to inflate
/// project totals).
fn build_record(file: &serde_json::Value) -> Option<CoverageRow> {
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
    Some(CoverageRow::from_summary(filename, summary))
}

/// FN-1 / TASK-1553 + PATTERN-3 / TASK-1558: push `record` onto `records`,
/// or overwrite the prior slot when its filename was already seen. Uses
/// `HashMap::entry` so the dedup path costs a single probe.
fn dedup_push(
    records: &mut Vec<CoverageRow>,
    idx_map: &mut std::collections::HashMap<String, usize>,
    record: CoverageRow,
    duplicate_count: &mut usize,
) {
    use std::collections::hash_map::Entry;
    match idx_map.entry(record.filename.clone()) {
        Entry::Occupied(slot) => {
            *duplicate_count += 1;
            records[*slot.get()] = record;
        }
        Entry::Vacant(slot) => {
            slot.insert(records.len());
            records.push(record);
        }
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
    let file_arrays: Vec<&Vec<serde_json::Value>> = data
        .iter()
        .map(|entry| {
            entry
                .get("files")
                .and_then(|f| f.as_array())
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
    for file in file_arrays.into_iter().flat_map(|f| f.iter()) {
        let Some(record) = build_record(file) else {
            continue;
        };
        dedup_push(
            &mut records,
            &mut filename_to_idx,
            record,
            &mut duplicate_count,
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

/// Run `cargo llvm-cov` and flatten its JSON output into per-file records.
///
/// ERR-1 / TASK-1057: with `--no-fail-fast`, `cargo llvm-cov` still exits
/// non-zero when one or more tests fail, but stdout contains a complete
/// llvm-cov JSON document for the passing slice of the workspace. Treat
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
#[must_use = "collect_coverage drives the coverage ingest; dropping the result throws the run away"]
pub fn collect_coverage(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error> {
    let output = run_cargo_llvm_cov(working_dir)?;
    if !output.status.success() {
        let parsed = serde_json::from_slice::<serde_json::Value>(&output.stdout).ok();
        let has_nonempty_data = parsed
            .as_ref()
            .and_then(|raw| raw.get("data").and_then(|d| d.as_array()))
            .is_some_and(|a| !a.is_empty());
        if has_nonempty_data {
            let tail = format_error_tail(&output.stderr, 5);
            let marker = format_cargo_exit(&output.status);
            tracing::warn!(
                exit = %marker,
                stderr_tail = %tail,
                "TASK-1057: cargo llvm-cov exited non-zero but stdout contains parseable JSON; \
                 continuing with partial coverage data (likely test failures with --no-fail-fast)"
            );
            return flatten_coverage_json(
                parsed.as_ref().expect("non-empty implies parsed is Some"),
            );
        }
        check_llvm_cov_output(&output)?;
    }
    let raw: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parsing llvm-cov JSON output")?;
    flatten_coverage_json(&raw)
}
