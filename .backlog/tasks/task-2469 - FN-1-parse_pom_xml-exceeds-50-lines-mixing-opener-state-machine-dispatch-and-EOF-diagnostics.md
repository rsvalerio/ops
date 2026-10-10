---
id: TASK-2469
title: 'FN-1: parse_pom_xml exceeds 50 lines, mixing opener state machine, dispatch, and EOF diagnostics'
status: Done
assignee: []
created_date: '2026-10-10 15:27'
updated_date: '2026-10-10 22:07'
labels:
  - code-review-rust
  - fn
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-java/about/src/maven/pom.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions-java/about/src/maven/pom.rs:parse_pom_xml'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:145`

**What**: `parse_pom_xml` (lines 145-234, ~90 lines incl. body of ~72) runs three distinct responsibilities in one function: the multi-line `<project ...>` opener state machine (lines 169-199), the started-line dispatch loop (lines 200-203), and three inline end-of-input `tracing::warn!` diagnostic blocks (lines 211-231). It also reaches 5 levels of nesting on the opener path: `for` -> `if !started` -> `if opener_pending` -> `if let Some((_, after_gt))` -> `if remainder.is_empty()` (lines 162-191).

**Why it matters**: FN-1 caps functions at 50 lines so each operates at one abstraction level. The Gradle sibling in this same crate already demonstrates the extraction: `gradle/mod.rs` factors its end-of-input diagnostics into a named `warn_unterminated` helper (gradle/mod.rs:235-243), while the Maven side inlines three warn blocks, so the two parsers drift structurally. Extracting the EOF-diagnostics block (and an early guard flattening the opener path) brings the function under the threshold and under 4 nesting levels. Note for triage: FN-1 acknowledges state-machine loops as a possible justified exception; the extraction target here is the diagnostics tail and opener nesting, not the loop itself.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_pom_xml operates at a single abstraction level: EOF diagnostics extracted to a named helper and the opener path flattened to <=4 nesting levels
- [x] #2 Function body is <=50 lines, or the remaining overage is documented as a justified state-machine exception

<!-- AC:END -->
