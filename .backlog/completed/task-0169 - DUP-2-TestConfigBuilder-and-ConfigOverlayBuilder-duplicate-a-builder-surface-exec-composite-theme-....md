---
id: TASK-0169
title: >-
  DUP-2: TestConfigBuilder and ConfigOverlayBuilder duplicate a builder surface
  (exec / composite / theme / ...)
status: Done
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-04-23 14:32'
labels:
  - rust-code-review
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/test_utils.rs:105-311

**What**: TestConfigBuilder (lines 104-194) and ConfigOverlayBuilder (lines 196-311) both offer nearly identical fluent methods — exec(name, program, args), composite(name, commands), theme(name), columns(n), show_error_detail(bool) — and each holds its own copy of the underlying IndexMap/OutputConfig*. The file even carries a "DUP-002" doc comment acknowledging the duplication and concluding a trait would be overkill. Two years of growth later the duplicated methods have each drifted (e.g., only TestConfigBuilder has stderr_tail_lines; only ConfigOverlayBuilder has custom_theme / enabled_extensions), so new contributors have to learn both.

**Why it matters**: DUP-2 / DUP-10 judgment. Test code tolerance is higher, but the drift is real and the existing acknowledgement note is no longer accurate. Options: (a) have TestConfigBuilder delegate to ConfigOverlayBuilder + a final apply-to-default step, (b) introduce a small trait CommandInsertion with default methods so both share the exec/composite/parallel_composite logic, (c) accept the duplication and at least add a regression test that every method mirrors between the two.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either unify the two builders behind a shared trait or document a concrete invariant keeping them in sync
- [ ] #2 Add at least one test that fails if a method exists on one builder and not the other
<!-- AC:END -->
