---
id: TASK-0500
title: >-
  SEC-23: resolve_spec_cwd silently allows absolute path escape under Deny
  policy
status: To Do
assignee:
  - TASK-0537
created_date: '2026-04-28 06:50'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:145`

**What**: When `spec_cwd` is absolute, `resolve_spec_cwd` returns it verbatim without consulting the workspace-escape policy. An absolute `cwd = "/etc"` under `Deny` still spawns there.

**Why it matters**: The `Deny` policy is documented as fail-closed for hooks where `.ops.toml` may be attacker-influenced. The absolute-path bypass means a malicious `cwd = "/"` is honoured even on the hook path. Containment check should apply to absolute paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 detect_workspace_escape runs against absolute spec_cwd as well
- [ ] #2 Hook test asserts Deny rejects absolute cwd outside workspace
<!-- AC:END -->
