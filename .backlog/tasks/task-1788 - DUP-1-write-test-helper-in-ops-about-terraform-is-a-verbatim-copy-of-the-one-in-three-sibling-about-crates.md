---
id: TASK-1788
title: >-
  DUP-1: write() test helper in ops-about-terraform is a verbatim copy of the
  one in three sibling about crates
status: Triage
assignee: []
created_date: '2026-08-27 11:23'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:429-434`

**What**: The `#[cfg(test)]` module opens with

```rust
fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}
```

byte-for-byte identical to the copies in `extensions-node/about/src/lib.rs`, `extensions-node/about/src/units.rs` and `extensions-python/about/src/units.rs` — six lines, over the DUP-1 threshold, four copies across the workspace.

**Why it matters**: `ops_about::test_support` exists precisely to retire this class of per-provider duplication (DUP-3 / TASK-0985), and this crate already lists `ops-extension` with the `test-support` feature in `[dev-dependencies]`, so the shared landing site is one import away. Any future tightening — propagating the `create_dir_all` error, switching to `expect` with a setup message per TEST-22, handling a path with no parent — has to land four times or the copies drift.

**Coordination**: TASK-1736 files the same helper against the two `extensions-node` copies and explicitly names this file as out of its scope, recommending `ops_about::test_support` as the landing site so all four copies retire at once. This task is the terraform half; if TASK-1736 lands the shared helper first, satisfying this one is a two-line change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The crate's test module uses a shared write() helper (preferably ops_about::test_support) instead of a local copy
- [ ] #2 cargo test -p ops-about-terraform passes unchanged
<!-- AC:END -->
