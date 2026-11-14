---
id: TASK-2210
title: 'PATTERN-1: an unterminated XML comment, <project> opener or section in pom.xml silently swallows the rest of the file'
status: Done
assignee: []
created_date: '2026-09-08 07:20'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - pattern
dependencies: []
parent_task_id: 'TASK-2238'
modified_files:
  - extensions-java/about/src/maven/pom.rs
priority: medium
ordinal: 121000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:99-152` (`parse_pom_xml`)

**What**: `parse_pom_xml` carries three pieces of unterminated-state that are never checked when the input runs out, and every one of them degrades to "empty POM" rather than "malformed POM":

1. `in_comment` — `strip_xml_comments` sets it on a `<!--` with no `-->` on the line. If the closing `-->` never arrives, every remaining line returns an empty string and is skipped. A single stray `<!--` anywhere near the top erases the whole manifest.
2. `opener_pending` — set by `is_project_open_start` for a multi-line `<project …` opener. If the closing `>` never arrives (or the file ends inside the attribute block), `started` stays `false`, every line is skipped by the `if !started` branch, and the function returns `Some(PomData::default())`.
3. The section state machine — `section` is only reset to `TopLevel` when the matching close tag (`</modules>`, `</developers>`, `</scm>`, `</licenses>`, or the tracked `Skip { close }`) is seen on its own line. A `<parent>` block whose `</parent>` is glued to other content, or a truncated file, leaves the parser parked in that section for the rest of the input, so every subsequent top-level `artifactId` / `version` / `name` is dropped.

In all three cases the function returns `Some(data)`, and `maven/mod.rs` folds it with `unwrap_or_default()`, so the provider cannot tell a malformed POM from an absent one — precisely the failure mode the crate's own lib.rs doc claims to have fixed ("a malformed manifest does not silently look like a missing one (TASK-0394)"). The read path does log, but the parse path emits nothing here.

Twin: TASK-2181 files the same shape for the Go stack (unterminated `use (` / `replace (` blocks). The Java instance is separate code with three distinct latch variables.

**Why it matters**: the About card silently falls back to the directory name with no version, license or modules, and nothing in the logs points at the malformed manifest. Given the parser is line-based by design, malformed input is the expected case to report, not to absorb.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 At end of input, parse_pom_xml detects a still-open XML comment, a still-pending <project ... opener, and a still-open section, and reports each via tracing::warn naming pom.xml and the unterminated construct
- [ ] #2 A pom.xml with an unterminated <!-- comment is covered by a test asserting the diagnostic path rather than a silently empty PomData
- [ ] #3 A pom.xml that ends inside a multi-line <project ... opener is covered by a test
- [ ] #4 A pom.xml with an unclosed <modules>/<scm>/<parent> section is covered by a test showing later top-level fields are either recovered or explicitly reported as dropped
<!-- AC:END -->
