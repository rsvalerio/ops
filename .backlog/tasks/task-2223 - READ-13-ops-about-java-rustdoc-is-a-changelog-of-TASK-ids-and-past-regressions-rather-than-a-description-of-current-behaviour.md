---
id: TASK-2223
title: 'READ-13: ops-about-java rustdoc is a changelog of TASK ids and past regressions rather than a description of current behaviour'
status: To Do
assignee: []
created_date: '2026-09-08 07:22'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions-java/about/src/lib.rs
  - extensions-java/about/src/maven/pom.rs
  - extensions-java/about/src/maven/mod.rs
  - extensions-java/about/src/gradle/mod.rs
  - extensions-java/about/src/gradle/lexer.rs
priority: low
ordinal: 130000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs` (24 `TASK-` citations), `extensions-java/about/src/gradle/mod.rs` (9), `extensions-java/about/src/gradle/lexer.rs` (9), `extensions-java/about/src/lib.rs` (4), `extensions-java/about/src/maven/mod.rs` (2), `extensions-java/about/Cargo.toml` (1)

**What**: 49 rule-id/TASK-id citations across ~1,300 lines of non-test source, and most of them sit in `///` docs that rustdoc publishes. The docs narrate the change history instead of the end state — e.g. the `SectionOutcome` enum doc spends its whole body describing what `match_section_open` "used to return" and which bug that caused; `split_at_unquoted_close_paren` documents "the previous hand-rolled byte scan treated `\"` as a closing quote"; `extract_quoted_list` explains what a removed second comment-strip pass used to truncate; `parse_include_line` lists five separate TASK ids; the crate-level `//!` block explains which task made the Maven provider use `unwrap_or_default`. `PomData::artifact_id` and `PomData::name` each spend three lines re-explaining the shared `try_set_once` policy.

Rule READ-13: docs should state what the code does now; the rationale for a past change belongs in the commit and the task, and the invariant it protects belongs in the test name.

Twins already filed on other crates: TASK-2191 (`ops-about-go`), TASK-2192 (`rust-loc`), TASK-2182 (`ops-deps`), TASK-2169 (text-fixers). This is the Java-stack instance; no file overlap.

**Why it matters**: a reader has to reconstruct current behaviour by subtracting the history from the prose, and the docs age badly — each already describes code that no longer exists. Some of these comments are genuinely load-bearing (the deliberate first-wins vs last-wins asymmetry between `parse_gradle_settings` and `parse_gradle_properties`, the `<scm>` vs `<licenses>` single-line asymmetry); those should survive the rewrite as statements of the rule, not as accounts of the bug that produced it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Published /// docs in the crate describe current behaviour and invariants without citing TASK ids or narrating superseded implementations
- [ ] #2 Every deliberate asymmetry currently justified only by a TASK citation (settings first-wins vs properties last-wins; <scm> rejecting duplicate openers while <licenses> accepts multiple children; comments stripped exactly once on the include path) is restated as a positive rule in the docs
- [ ] #3 Regression rationale that must be retained lives in the test name or a #[cfg(test)] comment, not in the public docs
<!-- AC:END -->
