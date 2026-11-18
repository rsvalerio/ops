---
id: TASK-2787
title: 'SEC-18: about machine probes rustc/cargo inside the checkout, where a rust-toolchain file can pick the binary'
status: To Do
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2857'
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-18:extensions/about/src/machine.rs:collect_machine_report'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->
**File**: `extensions/about/src/machine.rs:939-947` (`probe_host_target`), `:951-959` (`locate_workspace_root`), called from `collect_machine_report` (line 963)

**What**: The page runs `rustc -vV`, `rustc --print cfg` and `cargo locate-project` with the checkout as working directory (inherited cwd / `.current_dir(cwd)`). When `rustc`/`cargo` are rustup proxies, a checked-in `rust-toolchain.toml` selects the toolchain, and rustup's toolchain file accepts a `path` to a custom (local) toolchain, i.e. a repo-supplied binary. The same function explicitly refuses to run the configured `build.rustc-wrapper` because "this read-only page must not run a repo-supplied binary" (lines 993-995); the rustc/cargo probes do not honor that stated invariant.

**Why it matters**: Running `ops about machine` in an untrusted checkout could execute a binary the checkout controls. Verify against the rustup version in use (custom `path` toolchains and auto-install behavior); if the wider ops threat model already accepts running cargo in the checkout (e.g. the metadata provider), record that and narrow the comment at line 993 instead.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either the rustc/cargo probes run with toolchain-file overrides neutralized (e.g. RUSTUP_TOOLCHAIN pinned, or cwd outside the checkout where possible), or the threat-model decision is recorded
- [ ] #2 The comment beside sccache_stats states the invariant the module actually upholds
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
