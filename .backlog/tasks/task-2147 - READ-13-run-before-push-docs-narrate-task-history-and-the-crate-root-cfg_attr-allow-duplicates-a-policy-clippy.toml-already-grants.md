---
id: TASK-2147
title: 'READ-13: run-before-push docs narrate task history, and the crate-root cfg_attr allow duplicates a policy clippy.toml already grants'
status: To Do
assignee: []
created_date: '2026-09-08 07:02'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: low
ordinal: 60000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:3`, `:31`, `:77`, `:124`, `:251`, `:268`, `:290`, `:315`, `:478`, `:497`, `:519`, `:540`

**What**: Doc comments across the file document the backlog rather than the API, and one attribute documents a policy that is set elsewhere.

- `HOOK_SCRIPT` (:31-45) — a numbered essay on three "load-bearing properties" carrying two task IDs (`CL-3 / TASK-1911`, `SEC-11 / TASK-1906`) and counterfactuals about what users would otherwise do.
- `MAX_REF_UPDATE_LINES` (:77) and `classify_ref_updates` (:124) — inline `SEC-11:` rule tags in rendered docs.
- Eight test doc comments (:251, :268, :290, :315, :478, :497, :519, :540) open with a rule/task identifier: `CL-3 / TASK-1911`, `SEC-11 / TASK-1906`, `TASK-1906 AC#1`, `TEST-11 / TASK-0720`, `TEST-5 / TASK-1909` (three times). Two of them additionally narrate the sibling crate ("Mirrors the structural checks in run-before-commit so both crates stay in lockstep").
- `#![cfg_attr(test, allow(clippy::unwrap_used))]` (:3-8) with the comment "Test-only policy exception: assertions on known-good fixtures read better as `.unwrap()`". `clippy.toml` already sets `allow-unwrap-in-tests = true` workspace-wide, so the attribute grants nothing; it is a second, drifting copy of a policy that has one home. The sibling crate no longer carries this block (see TASK-2136, which records its removal there).

The enduring content is real and worth keeping — the hook must not depend on bash, `ops` is probed before it is exec'd, git's ref stream must never reach a spawned command, the classifier only skips on fully-understood input. It is the surrounding narration — task IDs, AC numbers, "so both crates stay in lockstep" — that a reader who joined after the decision cannot resolve and would delete verbatim.

**Why it matters**: `HOOK_SCRIPT`'s rationale, the two consts and `classify_ref_updates` all render in `cargo doc`, and the task IDs point at a backlog the reader has no access to. The redundant `cfg_attr` is worse than noise: it states a rule the build does not take from it, so editing it has no effect and reading it gives a false picture of where the panic policy lives — the exact drift the workspace lint comment in the root `Cargo.toml` warns against.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Item docs state the enduring property (POSIX sh only, ops probed before exec, git's ref stream never reaches a spawned command, the classifier only skips on fully-understood input) without narrating task history or the sibling crate
- [ ] #2 TASK-nnnn and rule identifiers are removed from /// and //! blocks in this file, or moved into body comments where they annotate a specific line
- [ ] #3 The crate-root cfg_attr(test, allow(clippy::unwrap_used)) block is removed, since clippy.toml's allow-unwrap-in-tests already covers it - or kept with a comment saying what it adds that clippy.toml does not
- [ ] #4 cargo clippy -p ops-run-before-push --all-targets and cargo doc still pass with no new warnings or broken intra-doc links
<!-- AC:END -->
