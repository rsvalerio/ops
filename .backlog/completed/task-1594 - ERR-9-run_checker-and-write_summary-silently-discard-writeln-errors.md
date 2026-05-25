---
id: TASK-1594
title: 'ERR-9: run_checker and write_summary silently discard writeln! errors'
status: Done
assignee:
  - TASK-1636
created_date: '2026-05-21 22:52'
updated_date: '2026-05-22 12:17'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:154,168`

**What**: Two output sites in the config-checkers extension silently drop I/O errors from `writeln!`:

- `lib.rs:154` — `writeln!(writer, "{label}: {}: {msg}", display.display()).ok();` discards a failed write of a per-file failure line. If the writer is a broken pipe or a closed file, the user gets neither the failure message nor any signal that output was lost; the run still returns `Ok(report)`.
- `lib.rs:168` — `let _ = writeln!(writer, "{label}: scanned {} file(s), {} failed", …)` in `write_summary` does the same for the final summary line.

The pattern is independent of, but worth fixing alongside, the `fs::read` swallow already filed as ERR-7 (TASK-1587).

**Why it matters**: The whole point of the checker is to surface failed files to the user. A silently dropped write turns a checker failure into an invisible failure: CI sees exit-code 0 (or whatever the caller does with the report) plus no diagnostic lines, and the developer has no way to know that a write error occurred. For library code that takes a `&mut dyn Write` from a caller, the contract should either propagate the `io::Error` or aggregate it into the returned report.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 writeln! errors at lib.rs:154 and lib.rs:168 are no longer silently discarded
- [x] #2 run_check_json/run_check_yaml/write_summary either propagate the io::Error or record it on the CheckerReport so callers can detect lost output
- [x] #3 Unit test exercises a writer that returns Err and verifies the failure is surfaced
<!-- AC:END -->
