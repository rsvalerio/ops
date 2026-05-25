---
id: TASK-1295
title: 'TEST-25: tools_cmd tests probe host rustfmt status via collect_tools'
status: Done
assignee:
  - TASK-1306
created_date: '2026-05-11 16:10'
updated_date: '2026-05-11 19:15'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/tools_cmd.rs:309-555`

**What**: Tests like `tools_list_shows_installed_and_missing`, `tools_check_all_installed`, and siblings call into `collect_tools`, which probes the host for `cargo-fmt` / rustfmt. Outcomes depend on whether the developer or CI image has rustfmt installed.

**Why it matters**: Hidden host coupling. Passes on dev machines that happen to have rustfmt; fails on a minimal-toolchain CI image. Makes the suite non-portable and the failures look like real regressions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Inject the ToolStatus source (trait object or fn pointer) into collect_tools so tests provide a deterministic status map
- [x] #2 Any remaining host-dependent test is gated behind #[ignore] or a feature with rationale
- [x] #3 Baseline CI image without rustfmt installed runs the cli test suite green
<!-- AC:END -->
