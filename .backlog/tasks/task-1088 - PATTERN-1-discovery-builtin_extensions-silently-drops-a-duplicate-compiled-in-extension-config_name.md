---
id: TASK-1088
title: >-
  PATTERN-1: discovery::builtin_extensions silently drops a duplicate
  compiled-in extension config_name
status: Done
assignee: []
created_date: '2026-05-07 21:31'
updated_date: '2026-05-08 06:49'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/discovery.rs:85`

**What**: `collect_compiled_extensions` returns `Vec<(&'static str, Box<dyn Extension>)>`, and `discovery::builtin_extensions` collects that into `HashMap<&'static str, Box<dyn Extension>>` — duplicate `config_name` keys (two extensions self-registering under the same name via `impl_extension!`) silently drop the earlier `Box`, with no warn breadcrumb.

**Why it matters**: The symmetric command/data-provider audit pipelines (TASK-0756, TASK-0876) explicitly avoid this; the discovery layer regressed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Duplicate compiled-in extension names emit a tracing::warn! naming both slots (or fail-closed in debug builds)
- [x] #2 Resolution is documented (last-write-wins / first-write-wins) and matches a stable order
- [x] #3 Add a unit test with two extensions sharing a config_name that asserts the audit warning fires
<!-- AC:END -->
