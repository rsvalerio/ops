---
id: TASK-2602
title: 'READ-13: write_file doc narrates the hoisting migration'
status: Done
assignee: []
created_date: '2026-10-10 20:49'
updated_date: '2026-10-10 21:56'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2623'
modified_files:
  - extensions/about/src/test_support.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/about/src/test_support.rs:write_file'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/test_support.rs:84`

**What**: The doc comment on `write_file` narrates how the helper came to exist: "The `about` extensions' `#[cfg(test)]` modules **each grew** a byte-identical six-line `write` helper for building tempdir fixtures. **Hoisting it here** gives the family one definition, so a future tightening ... lands once instead of drifting between copies."

**Why it matters**: READ-13: docs describe the end state. The enduring facts are: the helper writes fixture files, creates parents, and panics (via assert) on failure because it is fixture setup. The "grew / hoisting" migration story is PR-description content. Same pattern in the module-level docs at lines 9-29 ("It lives there rather than here because ... re-homing it the other way round would have made an extension a dev-dependency").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 write_file doc states behavior and failure semantics only; the hoisting/migration narration is removed
- [x] #2 Module docs state where the tracing harness lives and why without narrating the relocation history

<!-- AC:END -->
