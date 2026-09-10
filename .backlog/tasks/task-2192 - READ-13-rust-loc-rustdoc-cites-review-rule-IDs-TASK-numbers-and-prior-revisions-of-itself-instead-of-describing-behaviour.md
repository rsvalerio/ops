---
id: TASK-2192
title: 'READ-13: rust-loc rustdoc cites review rule IDs, TASK numbers and prior revisions of itself instead of describing behaviour'
status: Done
assignee: []
created_date: '2026-09-08 07:14'
updated_date: '2026-09-10 19:31'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions-rust/loc/src/lib.rs
  - extensions-rust/loc/src/views.rs
  - extensions-rust/loc/src/tests.rs
priority: low
ordinal: 105000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs` (lines ~90-160), `extensions-rust/loc/src/views.rs:20-25`, `extensions-rust/loc/src/tests.rs:6-14`

**What**: Public rustdoc in this crate reads as a review-and-change journal. Concrete sites:

- `lib.rs:105` `EXCLUDED_DIRS` — the doc spends a paragraph contrasting this list against "tokei's `TOKEI_DEFAULT_EXCLUDED`" and cites `(TASK-1974)`. The reader of `EXCLUDED_DIRS` needs the depth rule, not the argument that settled it against a sibling crate.
- `lib.rs:145` `collect_rust_loc` `# Errors` — "the cancellation check foreseen by the previous revision of this section (SEC-33 / TASK-2052)". The `# Errors` section documents a superseded revision of itself and a review-rule ID.
- `lib.rs:~168` and `lib.rs:~200` — inline comments open with `SEC-33 / TASK-2052:` as a header for otherwise-good explanations.
- `views.rs:20` `rust_loc_summary_view_sql` — the entire doc comment is `SEC-12 / ERR-5: ...` and `SEC-12 / TASK-1864: ...`; see the companion API-14 finding, which notes this leaves the item with no summary line at all.
- `views.rs:3` module doc header is `# Security (SEC-001)`.
- `tests.rs:8` `## Test isolation policy (TEST-17/TEST-18)`.

The *content* of these comments is good and should stay — the walk's degradation policy, the depth-cap rationale, and the lossy-path decision are all worth writing down. What should go is the framing: rule IDs and TASK numbers that only mean something to someone with the backlog open, and references to what a previous revision of the same doc used to say.

**Why it matters**: Same finding as TASK-2155 (cargo-update) and TASK-2099. These identifiers render into `cargo doc` output, they decay as tasks are closed and renumbered, and "the previous revision of this section" is unresolvable to any reader who is not doing archaeology on git history. The backlog task is the place for the audit trail; the doc comment is the place for the end state.

**Suggested fix**: Strip `TASK-####` and rule-ID prefixes from doc comments and keep the explanation they introduced, rewritten to describe the current behaviour in its own terms. Where the provenance genuinely matters, a git blame or the task itself carries it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No doc comment or section heading in the crate contains a TASK-#### reference or a review rule ID (SEC-33, SEC-12, ERR-5, TEST-17, TEST-18, SEC-001, READ-5)
- [x] #2 No doc comment refers to a previous revision of itself; the # Errors section on collect_rust_loc describes only the errors it returns today
- [x] #3 The substantive explanations (degradation policy, depth-cap rationale, excluded-dir depth rule, lossy-path decision) are preserved, rewritten to stand on their own without the sibling-crate comparison being required to understand the rule
- [x] #4 cargo doc -p ops-rust-loc builds cleanly and each affected item reads as a description of current behaviour

<!-- AC:END -->
