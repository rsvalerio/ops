---
id: TASK-1168
title: 'PATTERN-1: Hand-rolled pnpm YAML parser does not handle double-quoted escapes'
status: To Do
assignee:
  - TASK-1270
created_date: '2026-05-08 07:46'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:180-263`

**What**: `parse_pnpm_workspace_yaml` and helpers `split_inline_list` (236), `unquote` (257), `strip_trailing_yaml_comment` (270) implement a partial YAML 1.2 parser by hand. Double-quoted YAML scalars permit escape sequences (`\"\\\"\"`, `\"\\n\"`, `\"\\\\\"`, `\"\\u0023\"`) that the current parser does not interpret — `\"a\\\"\"` would split on the embedded `\"` mid-token because the in_double flag flips on every literal `\"`. Single-quoted scalars use `''` for an embedded apostrophe, also unhandled.

**Why it matters**: Parser is correct on common shapes pinned by tests but produces silently-wrong member globs on legitimate YAML. Long-term fix: delegate to a YAML crate; until then a guard rejecting backslash-containing scalars keeps failure visible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document explicitly which YAML subset is supported in the module-level rustdoc
- [ ] #2 When parse_pnpm_workspace_yaml encounters a scalar containing a backslash escape or a doubled apostrophe, emit tracing::debug! so operators see the unsupported shape
- [ ] #3 Add a test pinning current behaviour on "a\\\"b" and 'it''s' so future migration to a real YAML parser is a controlled change
<!-- AC:END -->
