---
id: TASK-0186
title: 'TEST-5: Stack::resolve has no test coverage'
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - TEST
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/stack.rs:38-45

**What**: Stack::resolve(config_stack: Option<&str>, workspace_root: &Path) -> Option<Self> is a public API used by both CommandRunner::new() and extensions::resolve_stack() (per its doc comment) but the stack::tests module (lines 130-378) has no test for it. Cases that would exercise real behavior: (1) config override wins over detect, (2) config_stack = Some("unknown") falls back to detect, (3) config_stack = None falls back to detect, (4) config_stack = Some("generic") returns Generic explicitly.

**Why it matters**: TEST-5. This is the single source of truth for stack-override precedence and drives both runtime behavior and init-template generation. Regressions here (e.g., accidentally swapping config_stack and detect priority) would silently pick the wrong stack defaults.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add unit tests covering the four resolve() precedence cases above
<!-- AC:END -->
