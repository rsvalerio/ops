---
id: TASK-2565
title: 'READ-13: CARGO_METADATA_ARGS doc narrates the testing strategy behind the constant'
status: Done
assignee: []
created_date: '2026-10-10 15:41'
updated_date: '2026-10-10 21:49'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2619'
modified_files:
  - extensions-rust/metadata/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/metadata/src/lib.rs:CARGO_METADATA_ARGS'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:230-236`

**What**: The const's doc is entirely about why it exists as a named constant for a test's benefit: "It is a named constant so a test can assert on the *value* the production call site uses. Asserting on the source text instead would test the formatter — `cargo fmt` rewrapping the list would fail while `--locked` was still passed, and deleting the call site would stay green so long as the literal survived anywhere in the file, a doc comment included."

**Why it matters**: READ-13 — this is a design journal entry defending the constant's shape against a testing alternative nobody needs to know about to use the API; it reads as process narration and goes stale as soon as the companion test changes (see the related TEST-32 finding on `run_cargo_metadata_arg_list_includes_locked` in `src/tests/wiring.rs`). What endures is one line: the argument list `cargo metadata` is invoked with.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Doc states the argument list handed to cargo without the paragraph defending the constant's existence for test-assertion purposes

<!-- AC:END -->
