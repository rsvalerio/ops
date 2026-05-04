---
id: TASK-0992
title: >-
  READ-7: tools::ToolStatus::Unknown variant is dead — never constructed but
  exposed in public Display contract
status: Done
assignee: []
created_date: '2026-05-04 21:59'
updated_date: '2026-05-04 23:51'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/lib.rs:60-65, 67-76`

**What**: `ToolStatus::Unknown` is declared with `#[allow(dead_code)]` and listed in the `Display` impl as `"unknown"`. No code path in the crate ever constructs the variant: probe failures (`run_probe_with_timeout` → None branches in probe.rs) all return `ToolStatus::NotInstalled`, and the comment at probe.rs:31 "reporting unknown/not-installed" is misleading because only `NotInstalled` is actually returned. The doc comment at line 53-54 ("CLI consumers fall back to format!('{}', status) for unknown variants") sells `Unknown` to consumers as a meaningful state, but it never appears in real output.

**Why it matters**: Downstream consumers reading the `tool_status` data provider (or matching on the enum) write defensive code for an `Unknown` arm that can never fire. Tools that legitimately failed to be probed get rendered as "not installed" — exactly inverting the operator-debug experience the variant was added to support. Either wire the variant up at the probe-failure call sites that the comment claims it represents, or remove it (and the `#[non_exhaustive]` already protects the API surface).

<!-- scan confidence: dead-variant verified by grepping the crate for `ToolStatus::Unknown` constructors -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either the probe-failure branches (probe.rs run_probe_with_timeout None, non-zero exit, etc.) return ToolStatus::Unknown — distinguishing 'probe failed' from 'tool genuinely not installed' — or the variant is removed and the misleading 'unknown/not-installed' comment in probe.rs is corrected
- [ ] #2 If kept: at least one regression test covers the Unknown construction path so it does not silently regress to NotInstalled
<!-- AC:END -->
