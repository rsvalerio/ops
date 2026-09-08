---
id: TASK-2205
title: >-
  TEST-5: invalid_root_pyproject_yields_no_units documents a warn assertion it
  does not make, and the warn it describes has no recovery field
status: To Do
assignee:
  - TASK-2241
created_date: '2026-09-08 07:19'
updated_date: '2026-09-08 10:56'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions-python/about/src/units.rs
priority: medium
ordinal: 118000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/units.rs:320`, `extensions-python/about/src/units.rs:101`

**What**: The test's doc comment states "the crate promises that a malformed root manifest degrades to *no units* ... **and says so via `tracing::warn!`** (TASK-0394 / TASK-0974). Both halves were previously unasserted." The body asserts only the first half:

```rust
fn invalid_root_pyproject_yields_no_units() {
    ...
    assert!(collect_units(dir.path()).is_empty());
}
```

There is no `capture_tracing` call — the warn half is still unasserted, exactly the gap the doc claims was closed. The `lib.rs` sibling (`invalid_pyproject_falls_back_to_directory_name_and_warns`) does capture the logs and assert on them.

Relatedly, the warn it should be asserting is the only diagnostic in the crate with no `recovery` field: `units.rs:101` emits `path` + `error` only, while every other warn in `lib.rs` and `units.rs` carries `recovery = "default-identity" / "skip-field" / "skip-author" / "skip-entry" / "keep-first"`.

**Why it matters**: A doc comment that overstates its assertions is worse than no comment — the next reviewer reads it as covered and deletes the warn without a failing test. The missing `recovery` field also breaks the structured-log contract the rest of the crate follows, so operators cannot filter Python About degradations uniformly.

<!-- scan confidence: verified by reading the test body -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 invalid_root_pyproject_yields_no_units captures WARN-level tracing and asserts the warn fires, names pyproject.toml, and states its recovery
- [ ] #2 The workspace-shape parse warn in read_workspace_members carries a recovery field consistent with the crate's other warns
- [ ] #3 Deleting the warn makes the test fail
<!-- AC:END -->
