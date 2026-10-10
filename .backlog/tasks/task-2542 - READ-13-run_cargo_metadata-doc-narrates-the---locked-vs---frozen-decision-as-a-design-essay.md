---
id: TASK-2542
title: 'READ-13: run_cargo_metadata doc narrates the --locked vs --frozen decision as a design essay'
status: To Do
assignee: []
created_date: '2026-10-10 15:37'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2619'
modified_files:
  - extensions-rust/metadata/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/metadata/src/lib.rs:run_cargo_metadata'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:207-220`

**What**: The `run_cargo_metadata` doc comment narrates the design journey behind the flag choice: "We prefer `--locked` over `--frozen` because the latter additionally forbids network access, which can break first-run metadata for fresh checkouts ... the operator-visible failure mode of `--frozen` is worse than the lockfile-mutation issue we're guarding against." That rejected-alternative comparison is a process artifact — it argues how the decision was made, not what the function does.

**Why it matters**: READ-13 — documentation describes the end state, not the journey that produced it. "Why we picked X over Y" essays go stale on the next change while looking authoritative, and they are meaningless to a reader using the API. The enduring, operator-relevant half of the doc (`--locked` is passed so the read-only ingestor cannot mutate `Cargo.lock`; it fails fast on drift) is legitimate and should stay; the `--frozen` comparison paragraph belongs in the PR description or an ADR. This matches the narration cleanup already landed for sibling crates (e.g. commits stripping history narration from theme/about/sqlite docs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc states what run_cargo_metadata does and the enduring invariant (--locked prevents lockfile mutation, fails fast on drift) without the --frozen rejected-alternative comparison

<!-- AC:END -->
