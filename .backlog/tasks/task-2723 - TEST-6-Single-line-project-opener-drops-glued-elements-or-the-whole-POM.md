---
id: TASK-2723
title: 'TEST-6: Single-line <project> opener drops glued elements or the whole POM'
status: Done
assignee: []
created_date: '2026-11-18 18:34'
updated_date: '2026-11-18 22:24'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2848'
modified_files:
  - extensions-java/about/src/maven/pom.rs
  - extensions-java/about/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'TEST-6:extensions-java/about/src/maven/pom.rs:accept_project_opener'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:197` (`accept_project_opener`), `extensions-java/about/src/maven/pom.rs:258` (`classify_project_opener`), `extensions-java/about/src/lib.rs:169` (fixture in `each_extension_registers_its_own_stack_identity_provider`)

**What**: The multi-line opener path re-dispatches text glued after the closing `>` (pinned by `parse_pom_multiline_project_opener_trailing_element_on_same_line`), but the single-line paths do not:

- `<project xmlns="..."><artifactId>x</artifactId>` classifies as `(true, true)`; `accept_project_opener` sets `started` and returns `None`, so the glued `<artifactId>` is silently dropped.
- `<project><artifactId>x</artifactId>...</project>` (compact/minified POM) never matches: the line is not exactly `<project>` and the char after `<project` is `>`, not whitespace, so `classify_project_opener` returns `(false, false)`, `started` never flips, and the whole POM parses to `PomData::default()` with **no** warning (`opener_pending` is false, section is `TopLevel`).

The edge case is untested, and the crate-level test in `lib.rs:169` writes exactly the compact shape (`<project><artifactId>demo-maven</artifactId><version>1.0</version></project>`) but only asserts `stack_detail == "Maven"`, which is constant, so it passes while the parser yields nothing.

**Why it matters**: Silent loss of name/version/description for a valid (if uncommon) POM layout, and a test fixture that looks like it exercises parsing but masks the defect. The module docs promise only "one element per line for top-level scalars", not that the opener line discards siblings.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A single-line opener (bare <project> or one with attributes) followed on the same line by elements dispatches the remainder, matching the multi-line opener path
- [x] #2 A compact one-line POM <project><artifactId>x</artifactId>...</project> yields artifact_id x, or at minimum emits a tracing::warn instead of silently returning an empty PomData
- [x] #3 Unit tests in pom.rs pin both single-line shapes; the lib.rs registration test asserts the parsed name, not only the constant stack_detail

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.08 (code-review-rust run, 2026-11-18).

Compact one-line POM now yields its scalars. Containers glued to the opener on that line are a documented limit; follow-up filed as TASK-2874.

<!-- SECTION:NOTES:END -->
