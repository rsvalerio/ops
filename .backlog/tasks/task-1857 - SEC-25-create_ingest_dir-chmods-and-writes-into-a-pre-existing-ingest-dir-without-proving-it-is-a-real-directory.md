---
id: TASK-1857
title: >-
  SEC-25: create_ingest_dir chmods and writes into a pre-existing ingest dir
  without proving it is a real directory
status: To Do
assignee:
  - TASK-2006
created_date: '2026-08-27 15:28'
updated_date: '2026-08-28 14:16'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/duckdb/src/sql/ingest/dir.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest/dir.rs:31-56` (`create_ingest_dir`)

**What**: The function documents an explicit co-tenant threat model ("a co-tenant on a multi-user system cannot tamper with staged data between collect and load") and hardens the leaf ingest dir to `0o700`. But the pre-existing branch trusts the path:

```rust
match std::fs::DirBuilder::new().recursive(false).mode(0o700).create(data_dir) {
    Ok(()) => {}
    Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}   // <-- trusts whatever is there
    Err(e) => return Err(e),
}
std::fs::set_permissions(data_dir, std::fs::Permissions::from_mode(0o700))?;
```

If `data_dir` (`<db path>.ingest`, i.e. `target/ops/data.duckdb.ingest` — a path under the world-traversable `target/`, which the sibling TASK-1000 note deliberately leaves at umask default) already exists as a **symlink to a directory**, `DirBuilder::create` fails with `AlreadyExists`, the code swallows it, and `set_permissions` then follows the symlink and chmods the *attacker-chosen target* to `0o700`. Every subsequent staged JSON write, sidecar write, and `read_json_auto()` load then operates inside that target. `set_permissions` is a path-based (not handle-based) call, so this is the classic SEC-25 check/act split: nothing between the create attempt and the chmod establishes that the path is a directory the current user owns.

The same applies to `std::fs::create_dir_all(parent)` above it for the intermediate components.

**Why it matters**: This is precisely the attack the `0o700` hardening exists to stop, and the doc comment asserts protection the code does not deliver. Downstream, the ingest dir feeds `read_json_auto('<path>')` (data trusted into DuckDB) and `read_workspace_sidecar` (a path that becomes a `data_sources` primary-key component), so control of the directory is control of ingested content.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 create_ingest_dir verifies a pre-existing data_dir is a real directory (symlink_metadata / is_symlink check, or an openat-style handle) before chmodding or writing into it
- [ ] #2 A pre-existing symlink at data_dir is rejected with a typed error instead of being silently adopted, and the permissions of the symlink target are left untouched
- [ ] #3 A unix test plants a symlink at the ingest dir path pointing at a separate directory and asserts create_ingest_dir errors and the target's mode is unchanged
- [ ] #4 The doc comment's co-tenant threat-model claim matches what the code enforces
<!-- AC:END -->
