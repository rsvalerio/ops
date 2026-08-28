---
id: TASK-1941
title: >-
  READ-4: the soft-fail log line and two doc comments still say the coverage
  JSON arrives on stdout, but it has gone to a temp file since --output-path was
  added
status: Done
assignee:
  - TASK-2000
created_date: '2026-08-27 15:47'
updated_date: '2026-08-28 15:53'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/parse.rs
  - extensions-rust/test-coverage/src/subprocess.rs
  - extensions-rust/test-coverage/src/tests.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:374`, `extensions-rust/test-coverage/src/subprocess.rs:87-88`, `extensions-rust/test-coverage/src/tests.rs:226`

**What**: `llvm_cov_argv` appends `--output-path <temp file>` and `collect_coverage` reads the report back with `std::fs::read(report.path())`. Nothing in the crate reads the JSON from `output.stdout` any more. Three places still describe the old behaviour:

1. `parse.rs:374` — the operator-facing warn emitted on the soft-fail path: "TASK-1057: cargo llvm-cov exited non-zero but stdout contains parseable JSON; continuing with partial coverage data". This is the one that matters; it is a structured log an operator reads while debugging a degraded coverage run, and it points them at a stream that carries the cargo test log, not the report.
2. `subprocess.rs:87-88` — the doc comment on `check_llvm_cov_output`: "when the exit is non-zero but stdout contains a parseable llvm-cov JSON document, `collect_coverage` treats it as a soft failure". The sibling doc on `run_cargo_llvm_cov` two functions above (lines 48-50) already says the opposite and correct thing ("writing the JSON report to `output_path` ... stdout stays small; stderr carries the test-run log"), so the file contradicts itself.
3. `tests.rs:226` and its doc comment at 178-185 — `non_zero_exit_without_files_surfaces_cargo_error` sets `stdout: br#"{"data":[{}]}"#` on its synthetic `Output` and the comment explains the case as "stdout carries `{"data": []}`". Production code never reads that field, so the fixture is inert. It passes for the right reason (the function only inspects `status` and `stderr`), but it teaches the next reader the wrong model of the data flow, which is how the other two comments got written.

`collect_coverage`'s own doc comment at parse.rs:319-323 is already correct ("the report file contains a complete llvm-cov JSON document"), which confirms the wording elsewhere is leftover rather than a deliberate shorthand.

**Why it matters**: the whole point of the `--output-path` change was that an ~8 MB report on stdout is silently truncated by `OPS_OUTPUT_BYTE_CAP` and destroys the coverage signal. A log line and a doc comment that still name stdout send an operator investigating a degraded run to look at the wrong place, and invite a future change to move the report back onto the stream the design deliberately abandoned.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The soft-fail warn in parse.rs names the report file as the source of the parseable JSON rather than stdout
- [x] #2 The check_llvm_cov_output doc comment in subprocess.rs describes the report file, and no longer contradicts the run_cargo_llvm_cov doc comment above it
- [x] #3 The inert stdout fixture and its comment in non_zero_exit_without_files_surfaces_cargo_error are removed or replaced with a comment stating that production reads the report file, not stdout
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The soft-fail warn now says "the JSON report file is parseable" and adds a report_path field. The check_llvm_cov_output doc names the --output-path report file and cross-references run_cargo_llvm_cov. The inert stdout fixture in non_zero_exit_without_files_surfaces_cargo_error is removed and replaced with a doc note stating production reads the report file, not stdout.
<!-- SECTION:NOTES:END -->
