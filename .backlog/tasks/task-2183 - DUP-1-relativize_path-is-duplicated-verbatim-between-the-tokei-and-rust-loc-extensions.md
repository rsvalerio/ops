---
id: TASK-2183
title: >-
  DUP-1: relativize_path is duplicated verbatim between the tokei and rust-loc
  extensions
status: To Do
assignee:
  - TASK-2242
created_date: '2026-09-08 07:13'
updated_date: '2026-09-08 10:57'
labels:
  - code-review-rust
  - duplication
dependencies: []
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
- [ ] #1 relativize_path exists in exactly one place in the workspace, in a crate both extensions already depend on
- [ ] #2 Both extensions call the shared helper; no local copy remains
- [ ] #3 The lossy-conversion rationale is documented once, on the shared definition
- [ ] #4 The existing tokei test relativize_path_replaces_invalid_utf8_with_replacement_char moves to (or is mirrored at) the shared helper, and both extensions' suites still pass
<!-- AC:END -->
