---
id: TASK-0178
title: >-
  FN-1: build_command nested cwd-resolution block mixes expansion,
  normalization, escape-check, and path build
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - FN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:81-103` (inside build_command)

**What**: The `match spec.cwd.as_deref()` block spans 22 lines at 3 levels of nesting (match → if ep.is_relative → if !normalized.starts_with) and handles four distinct concerns: variable expansion, relative→absolute joining, lexical normalization, workspace-root escape detection + warning. build_command itself is ~40 lines; the cwd handling alone is half of it.

**Why it matters**: FN-1 ("single abstraction level per function"). Extract `resolve_spec_cwd(spec_cwd: Option<&Path>, workspace_cwd: &Path, vars: &Variables) -> PathBuf` (or `Result<PathBuf>` if pursuing the SEC-14 fail-closed fix in TASK-0170). This makes build_command's intent (build a tokio::Command) clearer and makes the cwd resolution testable in isolation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract resolve_spec_cwd helper; build_command calls it for the single cwd value
<!-- AC:END -->
