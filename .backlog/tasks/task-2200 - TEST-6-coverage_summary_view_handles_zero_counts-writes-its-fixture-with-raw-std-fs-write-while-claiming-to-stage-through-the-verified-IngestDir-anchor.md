---
id: TASK-2200
title: >-
  TEST-6: coverage_summary_view_handles_zero_counts writes its fixture with raw
  std::fs::write while claiming to stage through the verified IngestDir anchor
status: To Do
assignee:
  - TASK-2241
created_date: '2026-09-08 07:15'
updated_date: '2026-09-08 10:56'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/tests/views.rs
priority: low
ordinal: 113000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests/views.rs:196`

**What**: the test carries the comment "SEC-25 / TASK-2054: stage through the same verified anchor `provide_via_ingestor` builds", opens an `IngestDir`, and then bypasses it:

```rust
std::fs::write(dir.entry_path("coverage_files.json"), &json_bytes).expect("write");
std::fs::write(dir.entry_path("coverage_workspace.txt"), "/test/workspace").expect("write workspace");
```

Every sibling stages through the descriptor instead — `dir.write_atomic(...)` in `src/tests/mod.rs::write_coverage_fixture`, in `src/tests/wiring.rs`, and in `ingestor.rs`'s inline `coverage_load_with_sample_data`. `entry_path()` hands back a bare path that is then resolved by name, which is exactly the by-name resolution the `IngestDir` handle exists to eliminate.

**Why it matters**: the test's setup no longer matches production's staging path, so it cannot catch a regression in whatever `write_atomic` does beyond a plain write (atomic rename, permissions, anchored resolution). It also reads as if it does — the comment claims the anchored behaviour it skips — which is worse than an unannotated shortcut for the next reader deciding whether the anchor is load-bearing here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 coverage_summary_view_handles_zero_counts stages both fixture files through IngestDir::write_atomic rather than std::fs::write on entry_path
- [ ] #2 The test still asserts the all-zero-row percentage case (0% not NaN) that distinguishes it from coverage_summary_view_empty_table_yields_zero_counts
<!-- AC:END -->
