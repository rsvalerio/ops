---
id: TASK-1815
title: >-
  FN-1: run_checker spans 87 lines and repeats the same record-and-report block
  three times
status: Triage
assignee: []
created_date: '2026-08-27 11:32'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:211-297` (`run_checker`)

**What**: the function body runs 87 lines (211-297) against the 50-line guideline, and mixes four abstraction levels in one scope: discovery + context wrapping, extension filtering, path relativisation, a `metadata` size gate, file I/O, parser invocation, writer formatting, and report bookkeeping.

The length is driven by one repeated shape. This block appears three times, differing only in the message prefix (DUP-1 — identical 5+ line blocks):

```rust
report.files_scanned = report.files_scanned.saturating_add(1);
let msg = format!("metadata: {e}");                        // "read: {e}" / err.to_string()
writeln!(writer, "{label}: {}: {msg}", display.display())
    .with_context(|| format!("{label}: writing failure line failed"))?;
report.files_failed.push(FailedFile { path: display, message: msg });
continue;
```

at `lib.rs:257-267` (metadata error), `lib.rs:271-283` (read error), and `lib.rs:284-293` (parse error, the same body minus the `continue`). Three copies of the counter bump, the `writeln!` + identical `with_context` closure, and the `FailedFile` push — so a change to the failure-line format or to the bookkeeping has to be made in three places and can silently drift in one.

**Why it matters**: FN-1 / DUP-1. The duplication is not cosmetic here: the three copies are exactly where the report's counting semantics are decided, and they already disagree with the type's documented meaning (see the ERR-2 finding on `FailedFile` / `files_scanned`). Collapsing them to one helper is what makes that class of bug a one-line fix instead of a three-site audit.

**Fix shape**: extract `fn record_failure(report: &mut CheckerReport, writer: &mut dyn Write, label: &str, display: PathBuf, msg: String) -> anyhow::Result<()>` and call it from all three sites; lift the size gate + read into a `read_candidate(path, max_bytes) -> Result<Option<Vec<u8>>, ...>` helper so the loop body reads as discovery -> filter -> read -> check -> record. That takes the function under the guideline without inventing an abstraction that is not already implicit in the code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The three record-and-report blocks in run_checker are replaced by a single shared helper
- [ ] #2 run_checker's body is at or under the 50-line guideline, with the size gate and read extracted to a named helper
- [ ] #3 Existing behaviour is unchanged: the emitted failure lines, files_scanned/files_skipped counts, and writer-error propagation still match the current tests
<!-- AC:END -->
