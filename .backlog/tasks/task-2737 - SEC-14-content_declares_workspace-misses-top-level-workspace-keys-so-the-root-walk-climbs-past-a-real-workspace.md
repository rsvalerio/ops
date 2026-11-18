---
id: TASK-2737
title: 'SEC-14: content_declares_workspace misses top-level workspace keys, so the root walk climbs past a real workspace'
status: Done
assignee: []
created_date: '2026-11-18 18:37'
updated_date: '2026-11-18 22:28'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2850'
modified_files:
  - extensions-rust/cargo-toml/src/workspace_root.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-14:extensions-rust/cargo-toml/src/workspace_root.rs:content_declares_workspace'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/workspace_root.rs:455` (`content_declares_workspace`), consumed by `manifest_declares_workspace` (`:425`) in both the lenient and strict ancestor walks.

**What**: The line scanner only recognises a `[workspace]` / `[workspace.*]` table *header*. TOML (and therefore Cargo) also accepts the workspace table as a top-level key before any header:

```toml
workspace = { members = ["crates/*"] }
# or
workspace.members = ["crates/*"]
workspace.resolver = "2"
```

Line 463 skips every line that does not start with `[`, so both forms return `false`. `nested.rs` does not have this gap: it parses the manifest and checks `table.get("workspace").is_some_and(toml::Value::is_table)`, so the two discovery paths disagree on the same file.

**Why it matters**: The function's own doc says a false negative is security-relevant: the walk treats the real root as a plain member, keeps climbing, and returns the nearest *ancestor* `Cargo.toml` that declares `[workspace]`, which is attacker-plantable (see the threat model on `find_workspace_root`). Every downstream provider (`cargo_toml`, units, coverage, deps) then targets the wrong workspace. When no ancestor declares one, the root is still found only through the `RecordFirst` fallback, by accident.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 content_declares_workspace returns true for a top-level inline `workspace = { ... }` and for a top-level dotted `workspace.<key> = ...` appearing before the first table header
- [x] #2 A dotted `workspace.x` key inside another table (e.g. under [package] or [dependencies]) still returns false
- [x] #3 find_workspace_root and find_workspace_root_strict have tests that select an inline-declared workspace root over an ancestor declaring [workspace]

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.12 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
