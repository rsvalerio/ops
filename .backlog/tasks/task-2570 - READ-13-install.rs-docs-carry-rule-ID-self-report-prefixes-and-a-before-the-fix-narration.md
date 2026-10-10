---
id: TASK-2570
title: 'READ-13: install.rs docs carry rule-ID self-report prefixes and a before-the-fix narration'
status: Done
assignee: []
created_date: '2026-10-10 15:42'
updated_date: '2026-10-10 22:14'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/hook-common/src/install.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/hook-common/src/install.rs:install module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:634-638`; rule-ID prefixes at `56,87,135,157,182,308,422,535,563,608,634,683,722,819,854,899,930,990,1025,1060,1082,1116,1155,1215`

**What**: One test doc narrates the fix journey: 'Before the fix the installer read it, found no legacy marker, and told the operator to remove a hook not installed by ops — about a file ops itself had half-written, wedging reinstall' (634-638). Twenty-four `///` docs across production items (`reject_symlinked_hook`, `write_new_hook`, `ExistingHook::Partial`, `read_existing_hook_capped`, `upgrade_legacy_hook`, `stage_hook_payload`) and test fns open with review-rule self-report prefixes (`/// SEC-25: ...`, `/// SEC-33: ...`, `/// PATTERN-1: ...`, `/// ERR-13: ...`). The crate-level cleanup applied to crates/theme in e5b5443f (strip tag, keep prose) has not reached this file.

**Why it matters**: READ-13 — process artifacts that narrate which guidelines a change followed are meaningless to a reader using the API and silently go stale; the crash-safety and refusal contracts themselves are the enduring content and stay.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The before-the-fix narration is removed; the truncated-artefact classification contract stays
- [x] #2 No doc comment in the file opens with a rule-ID prefix
- [x] #3 cargo test -p ops-hook-common passes

<!-- AC:END -->
