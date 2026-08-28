---
id: TASK-1949
title: >-
  ERR-13: both reads of the llvm-cov report drop the path — one discards the IO
  error entirely, the other reports 'reading llvm-cov JSON report' with no file
  named
status: Done
assignee:
  - TASK-2000
created_date: '2026-08-27 15:49'
updated_date: '2026-08-28 15:53'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/parse.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:356-358` and `extensions-rust/test-coverage/src/parse.rs:389-391`

**What**: `collect_coverage` reads the temp report file twice, on the two exit paths, and neither read names the file it failed on.

Line 389, the success path:

    let bytes = std::fs::read(report.path()).context("reading llvm-cov JSON report")?;

The context says what was being attempted but not which path, so an operator sees "reading llvm-cov JSON report: Permission denied" with nothing to inspect. The path is a `tempfile` under the system temp directory, and naming it is exactly what tells the operator whether TMPDIR is full, read-only, mounted noexec, or swept by a cleaner mid-run — the failures this line actually sees in practice. The same applies to the `serde_json::from_slice(...).context("parsing llvm-cov JSON output")?` on the next line: a parse failure on an ~8 MB document with no path leaves nothing to look at.

Line 356, the soft-fail path:

    let parsed = std::fs::read(report.path())
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());

Here the IO error is discarded outright by `.ok()`, which collapses three different situations into one: cargo wrote no report, cargo wrote a report we could not read, and cargo wrote a report that is not JSON. All three fall through to `check_llvm_cov_output`, which reports the cargo exit code — correct as the headline, but it means a genuine filesystem problem is reported to the operator as a cargo failure with no trace that the report read was even attempted. A `tracing::debug!` or `warn!` breadcrumb naming the path and the IO error before falling through would keep the cargo exit as the primary error while leaving the real cause discoverable.

ERR-13's stated fix ordering applies: attach `.with_context(|| format!("reading llvm-cov JSON report {}", report.path().display()))` at the two callsites, since the crate has only these two filesystem reads and does not warrant swapping the module for `fs_err`.

**Why it matters**: these errors surface through `ops about --refresh` on operator machines and in CI, where the whole diagnostic is the string in the log. "No such file or directory" with no path is the specific failure ERR-13 exists to prevent, and the discarded error on the soft-fail path is worse — it is not merely pathless, it is gone.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The success-path report read attaches the report file path to its error context, and so does the JSON parse that follows it
- [x] #2 The soft-fail path no longer discards the IO error silently: a failed report read emits a breadcrumb naming the path and the underlying error before falling through to the cargo exit error
- [x] #3 The cargo exit error remains the headline error on the soft-fail path; the breadcrumb does not replace or mask it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Both success-path reads use with_context naming report.path(). The soft-fail read no longer uses .ok(): the Err arm emits a warn breadcrumb with report_path and the IO error, then falls through, leaving check_llvm_cov_output as the headline error (pinned by tests/collect.rs::collect_coverage_missing_report_falls_through_to_cargo_error).
<!-- SECTION:NOTES:END -->
