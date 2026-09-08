---
id: TASK-2189
title: >-
  READ-12: severity_is_actionable warns once per entry while its doc promises a
  one-off warning
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-08 07:14'
updated_date: '2026-09-08 11:01'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/deps/src/lib.rs
priority: low
ordinal: 102000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:298`

**What**: the `SeverityClass::Unknown` arm of `severity_is_actionable` fires a `tracing::warn!` on every call. `has_issues` calls it once per advisory, per license, per ban and per source entry, so a cargo-deny schema change that renames a severity produces one warn line per finding — on a workspace with 28 duplicate-crate diagnostics that is 28 identical lines.

Both doc blocks above it claim otherwise: `severity_is_actionable` says "fire a `tracing::warn!` so schema drift surfaces in logs", and `has_issues` says "unknown severities fire a **one-off** `tracing::warn!`". The renderer side got this right — `format::severity_row` guards the same warning with a `warned_unknown` flag so it emits at most once per section — so the two sides of the same drift signal behave differently.

Additionally the message body embeds the task id as prose (`"TASK-0601: unknown cargo-deny severity treated as actionable…"`) while the severity is already emitted as a structured field.

**Why it matters**: the operator-facing signal for schema drift is the thing that floods. Repeating an identical line per entry buries the rest of the run's output and makes the log useless for the diagnosis it exists to support, and the doc comment tells a reader the opposite of what the code does.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the unknown-severity warning is emitted at most once per has_issues call, matching the warned_unknown behaviour in format::severity_row
- [ ] #2 the doc comments on severity_is_actionable and has_issues describe the actual emission behaviour
- [ ] #3 a test asserts that a report carrying several unknown-severity entries produces a single warn line
<!-- AC:END -->
