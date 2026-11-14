---
id: TASK-2390
title: 'SEC-11: create-review-tasks validate() accepts Unicode format/bidi characters that is_control() does not catch, so provider strings can still reorder the terminal report'
status: Done
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 15:59'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2422'
modified_files:
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-11:extensions/create-review-tasks/src/lib.rs:validate_field'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:243` (`validate_field`)

**What**: The boundary check rejects only `char::is_control()` (Unicode Cc). Format characters (Cf) such as U+202E RIGHT-TO-LEFT OVERRIDE, U+2066..U+2069 isolates, U+200B zero-width space and U+FEFF pass. Those strings come from repository-controlled data (each member's `[package].name`) and reach the stdout report (`report`), the YAML title and the filename slug. The doc comment states the goal is that nothing can "rewrite the operator's terminal around the run report"; bidi overrides can visually reorder the report line, which is the "Trojan Source" shape.

**Why it matters**: The validator's stated threat model is incompletely covered, and a hostile repo under review can make the report show a title different from the one written. Severity is low, since it affects display only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 validate_field also rejects Unicode format characters (bidi overrides/isolates, zero-width, BOM) or restricts to a printable allowlist
- [x] #2 A unit test feeds a target name containing U+202E and asserts the run fails before any file is written

<!-- AC:END -->
