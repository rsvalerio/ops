---
id: TASK-0424
title: 'ARCH-11: workspace binary deps not centralized in cli/runner/theme manifests'
status: To Do
assignee:
  - TASK-0538
created_date: '2026-04-28 04:41'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `crates/cli/Cargo.toml` — clap = "4", tracing-subscriber = "0.3", inquire = "0.9", toml = "1", assert_cmd = "2", predicates = "3", serial_test = "3"
- `crates/runner/Cargo.toml` — indicatif = "0.18", proptest = "1"
- `crates/theme/Cargo.toml` — serial_test = "3", toml = "1"

**What**: TASK-0413 closed the same drift for the extensions/* crates but left the binary-side crates (cli/runner/theme) still pinning common deps inline. `serial_test = "3"` and `toml = "1"` already appear in two different crate manifests; a future bump must remember to touch every site.

**Why it matters**: ARCH-11 — single-point CVE / version upgrades, prevents silent drift across sibling crates, and centralizes test-only versions just like prod versions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add clap, indicatif, inquire, tracing-subscriber, toml, serial_test, assert_cmd, predicates, proptest to [workspace.dependencies] in root Cargo.toml
- [ ] #2 Member crates use dep = { workspace = true } (with features overrides where needed)
- [ ] #3 cargo metadata shows a single resolved version per dep across the workspace
<!-- AC:END -->
