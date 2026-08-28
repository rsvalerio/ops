//! `collect_coverage`'s soft-fail / hard-fail policy.
//!
//! TEST-6 / TASK-1938: a real `collect_coverage` run executes the whole
//! workspace suite under instrumentation with a 15-minute timeout, so these
//! tests drive [`collect_coverage_with`] and inject a cargo runner that
//! writes a synthetic report to the `--output-path` file and returns a
//! synthetic [`Output`], exactly as cargo would.

use crate::parse::has_parseable_coverage_data;

// Everything below drives a synthetic `Output`, which these tests build via
// `ExitStatusExt::from_raw` — a unix-only trait. The tests that need it are
// `#[cfg(unix)]`, so their imports must be too, or a non-unix build warns on
// three unused imports. `has_parseable_coverage_data` above is exercised by
// the one test that is not gated, so it stays unconditional.
#[cfg(unix)]
use crate::parse::collect_coverage_with;
#[cfg(unix)]
use crate::subprocess::check_llvm_cov_output;
#[cfg(unix)]
use std::process::Output;

/// A cargo runner double: writes `report` to the `--output-path` file and
/// exits with `raw_status` (a wait status, as `ExitStatusExt::from_raw`
/// takes: `1 << 8` is exit code 1).
#[cfg(unix)]
fn runner_writing(
    report: &'static str,
    raw_status: i32,
    stderr: &'static str,
) -> impl FnOnce(&std::path::Path, &str) -> Result<Output, ops_core::subprocess::RunError> {
    use std::os::unix::process::ExitStatusExt;
    move |_working_dir, output_path| {
        std::fs::write(output_path, report).expect("runner writes report");
        Ok(Output {
            status: std::process::ExitStatus::from_raw(raw_status),
            stdout: Vec::new(),
            stderr: stderr.as_bytes().to_vec(),
        })
    }
}

const GOOD_REPORT: &str = r#"{"data":[{"files":[
    {"filename":"src/main.rs","summary":{"lines":{"count":10,"covered":5,"percent":50.0}}}
]}]}"#;

/// TEST-6 / TASK-1938: the success path — cargo exits 0, the report file is
/// read, parsed, and flattened into per-file rows.
#[cfg(unix)]
#[test]
fn collect_coverage_success_flattens_report_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let rows = collect_coverage_with(dir.path(), runner_writing(GOOD_REPORT, 0, ""))
        .expect("zero exit with a parseable report must succeed");
    let arr = rows.as_array().expect("flatten output is an array");
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["filename"], "src/main.rs");
    assert_eq!(arr[0]["lines_count"], 10);
}

/// ERR-1 / TASK-1057, TEST-6 / TASK-1938: the soft-fail demotion — cargo
/// exits non-zero (a failing test under `--no-fail-fast`) but the report
/// holds a complete document, so the partial rows are returned rather than
/// an error.
#[cfg(unix)]
#[test]
fn collect_coverage_demotes_non_zero_exit_with_parseable_report() {
    let dir = tempfile::tempdir().expect("tempdir");
    let rows = collect_coverage_with(
        dir.path(),
        runner_writing(GOOD_REPORT, 1 << 8, "error: 1 test failed"),
    )
    .expect("non-zero exit with a parseable report must be demoted to a warn");
    let arr = rows.as_array().expect("flatten output is an array");
    assert_eq!(
        arr.len(),
        1,
        "partial coverage must survive the soft-fail demotion"
    );
}

/// ERR-1 / TASK-1557 + TASK-1597, TEST-6 / TASK-1938: the hard-fail
/// fall-through — the predicate rejects the report, so the surfaced error
/// names the cargo exit rather than a schema-shape parse failure.
#[cfg(unix)]
#[test]
fn collect_coverage_non_zero_exit_with_rejected_report_surfaces_cargo_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = collect_coverage_with(
        dir.path(),
        runner_writing(
            r#"{"data":[{}]}"#,
            101 << 8,
            "error: could not compile `ops`",
        ),
    )
    .expect_err("a report without files must not be demoted");
    let msg = err.to_string();
    assert!(
        msg.contains("cargo llvm-cov exited with status 101"),
        "cargo exit must be the headline error, got: {msg}"
    );
    assert!(
        !msg.contains("'files'"),
        "must not surface a schema-shape parse error, got: {msg}"
    );
}

/// ERR-13 / TASK-1949 + TEST-6 / TASK-1938: an unreadable (here: removed)
/// report on the non-zero path falls through to the cargo error, not to a
/// JSON problem. The breadcrumb naming the path is emitted at `warn`; the
/// cargo exit stays the headline.
#[cfg(unix)]
#[test]
fn collect_coverage_missing_report_falls_through_to_cargo_error() {
    use std::os::unix::process::ExitStatusExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let err = collect_coverage_with(dir.path(), |_working_dir, output_path| {
        std::fs::remove_file(output_path).expect("remove the report the tempfile pre-created");
        Ok(Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: Vec::new(),
            stderr: b"error: build failed".to_vec(),
        })
    })
    .expect_err("a missing report must not be demoted");
    let msg = err.to_string();
    assert!(
        msg.contains("cargo llvm-cov exited with status 1"),
        "cargo exit must remain the headline error, got: {msg}"
    );
    assert!(
        !msg.contains("JSON"),
        "a missing report must not be reported as a JSON problem, got: {msg}"
    );
}

/// ERR-13 / TASK-1949: on the success path the report-read and parse errors
/// name the file, so an operator hitting a TMPDIR problem has something to
/// inspect.
#[cfg(unix)]
#[test]
fn collect_coverage_success_path_parse_error_names_the_report_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let err = collect_coverage_with(dir.path(), runner_writing("not json at all", 0, ""))
        .expect_err("an unparseable report on a zero exit is a hard error");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("parsing llvm-cov JSON report /"),
        "the report path must appear in the error context, got: {msg}"
    );
}

/// DUP-1 / TASK-1929: the soft-fail predicate has one home in `parse.rs`,
/// and this guard binds to it. Previously the test declared its own copy of
/// the predicate body and asserted against that, so any mutation of the
/// production predicate left the suite green.
///
/// ERR-1 / TASK-1557: an empty `data[]` must fall through to the cargo error
/// path — otherwise `flatten_coverage_json` reports "'data' array is empty",
/// hiding the real cause (compile error, missing toolchain, OOM kill).
/// ERR-1 / TASK-1597: a `data[]` entry without a `files` array must fall
/// through for the same reason.
#[test]
fn soft_fail_predicate_rejects_empty_data_array() {
    let empty = serde_json::json!({"data": []});
    let populated = serde_json::json!({"data": [{"files": []}]});
    let no_files = serde_json::json!({"data": [{}]});
    assert!(
        !has_parseable_coverage_data(&empty),
        "empty data[] must fall through to cargo error path so the operator sees the real cause"
    );
    assert!(
        has_parseable_coverage_data(&populated),
        "non-empty data[] with files must drive the soft-fail warn-and-continue branch"
    );
    assert!(
        !has_parseable_coverage_data(&no_files),
        "data[] entry without files must fall through to cargo error path"
    );
}

/// ERR-1 / TASK-1597: when the predicate rejects (e.g. data entry lacks
/// files), `check_llvm_cov_output` surfaces the cargo exit + stderr tail,
/// not a schema-shape parse error from `flatten_coverage_json`.
///
/// READ-4 / TASK-1941: the synthetic `Output` carries no stdout on purpose.
/// Production reads the llvm-cov report from the `--output-path` file, never
/// from stdout, and this helper only inspects `status` and `stderr`.
#[cfg(unix)]
#[test]
fn non_zero_exit_without_files_surfaces_cargo_error() {
    use std::os::unix::process::ExitStatusExt;
    let output = Output {
        status: std::process::ExitStatus::from_raw(256),
        stdout: Vec::new(),
        stderr: b"error: something failed".to_vec(),
    };
    let err = check_llvm_cov_output(&output).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("cargo llvm-cov exited with"),
        "should surface cargo error, got: {msg}"
    );
    assert!(
        !msg.contains("'files'"),
        "should not mention files schema, got: {msg}"
    );
}
