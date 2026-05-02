---
id: TASK-0939
title: >-
  DUP-1: tests/integration.rs and crates/cli/tests/integration.rs duplicate ~75%
  of test bodies
status: Done
assignee: []
created_date: '2026-05-02 15:51'
updated_date: '2026-05-02 16:10'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `tests/integration.rs:1-449` vs `crates/cli/tests/integration.rs:1-544`

**What**: The workspace-root `tests/integration.rs` and the cli-crate `crates/cli/tests/integration.rs` are forks of the same file. `diff` reports they share the helper layer (`with_ops_toml`, `temp_dir`, `write_ops_toml`, `read_ops_toml`, `ops()`) verbatim and re-implement the same `cli_version`, `cli_help`, `cli_init_*`, `cli_run_*`, `cli_dry_run_*`, `cli_theme_*`, `cli_run_command_with_timeout`, `cli_with_invalid_ops_d_file` cases. The cli-crate copy has additional tests on top, suggesting it is the active integration-test home and the workspace-root copy is a fossil.

Key tells that this is real duplication, not two distinct test surfaces:
- Both build the same `Command::cargo_bin("ops")` binary.
- Both use identical helper signatures and identical body shape (`with_ops_toml(r#"..."#, |path| { ops().arg(...)... })`).
- The workspace-root file even carries `#![allow(deprecated)]` "for `set_var`/`remove_var`" but never references either function, evidence of stale copy-paste from a former version.

**Why it matters**: Two CI runs for one logical surface — every CLI behaviour change has to be edited in both places or one drifts silently. DUP-10 grants higher tolerance to tests, but TEST-12 still flags *truly redundant* cases ("identical logic, same paths, copy-paste with trivial differences"), which is exactly what `diff -u` shows here.

Recommended action: pick the canonical home (the cli-crate copy is the natural one — the binary lives in `crates/cli`), delete the workspace-root file, and remove the dead `#![allow(deprecated)]` from whichever copy survives.

<!-- scan confidence: candidates to inspect -->
- `tests/integration.rs:1-449` (workspace-root, has stale `#![allow(deprecated)]`)
- `crates/cli/tests/integration.rs:1-544` (superset, active)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Only one integration.rs covers the cli-binary integration surface (workspace-root file removed, or the cli-crate file is removed and tests/Cargo.toml absorbs the missing cases)
- [ ] #2 The surviving file does not carry a #![allow(deprecated)] crate attribute unless it actually uses a deprecated API
- [ ] #3 ops verify / ops qa stay green after the deduplication
<!-- AC:END -->
