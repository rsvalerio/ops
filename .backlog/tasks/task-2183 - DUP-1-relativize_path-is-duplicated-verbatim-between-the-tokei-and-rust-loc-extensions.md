---
id: TASK-2183
title: 'DUP-1: relativize_path is duplicated verbatim between the tokei and rust-loc extensions'
status: Done
assignee: []
created_date: '2026-09-08 07:13'
updated_date: '2026-09-10 16:21'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2242'
modified_files:
  - extensions-rust/loc/src/lib.rs
  - extensions/tokei/src/lib.rs
priority: low
ordinal: 96000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs:396` and `extensions/tokei/src/lib.rs:422`

**What**: Both extensions carry a byte-identical helper:

```rust
fn relativize_path(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}
```

Each copy also carries its own multi-paragraph rationale for why the lossy conversion is acceptable (rust-loc's doc explicitly says "for the same reason documented on the `tokei` extension's `relativize_path`", i.e. it is a copy that points at the original for its justification). Both crates already depend on `ops-duckdb`, and both feed the result into a JSON sidecar consumed by a DuckDB table, so the policy the doc comments describe — lossy is fine because the column is display-and-join-only, never round-tripped to disk or interpolated into SQL — is one shared policy expressed twice.

**Why it matters**: The two copies encode a security-relevant decision (when a non-UTF-8 path may be lossily converted). A future change to that policy has to find both sites, and the rust-loc copy has already delegated its justification to the tokei copy, so the pair can silently diverge from a doc that no longer applies. The sibling pattern has bitten this codebase before — see TASK-2162, where a duplicated helper across two extensions diverged on symlink handling.

**Suggested fix**: Move it to `ops_duckdb::sql` (or the nearest shared crate both already depend on) as one documented public helper, and have both extensions call it. Keep the rationale on the single definition.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 relativize_path exists in exactly one place in the workspace, in a crate both extensions already depend on
- [x] #2 Both extensions call the shared helper; no local copy remains
- [x] #3 The lossy-conversion rationale is documented once, on the shared definition
- [x] #4 The existing tokei test relativize_path_replaces_invalid_utf8_with_replacement_char moves to (or is mirrored at) the shared helper, and both extensions' suites still pass

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shared helper added as ops_duckdb::sql::relativize_path with the merged lossy-conversion rationale (READ-5 / TASK-0504 wording from the tokei copy, which the rust-loc copy already deferred to); both extensions call it, local copies and their duplicated doc comments removed. The lossy-contract test moved verbatim from extensions/tokei/src/tests.rs to the sql module tests (adapted only to call the shared fn). ops-duckdb (218), ops-tokei (56), ops-rust-loc (44) tests green; clippy clean.
<!-- SECTION:NOTES:END -->
