---
id: TASK-2228
title: 'READ-13: ops-about-terraform''s docs narrate 80 task IDs and past bug histories instead of describing current behaviour'
status: To Do
assignee: []
created_date: '2026-09-08 07:22'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 134000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs` (crate docs `:1`-`:27`, and most `///` blocks — e.g. `:130`, `:203`, `:243`, `:317`, `:568`, `:731`)

**What**: The file contains 80 `TASK-NNNN` references and ~20 "pre-fix / used to / previously" clauses in doc comments. The docs are a changelog of the crate's review history rather than a description of what the code does. Representative:

- `fallback_tf_paths` (`:130`-`:148`): 19 lines of doc, of which the first sentence is the behaviour and the rest is three separate `CL-3 / TASK-0852`, `ERR-1 / TASK-1772`, `PERF-3 / TASK-1782` incident write-ups.
- `extract_required_version` (`:203`-`:217`): "The three correctness bugs this shape replaced (brace-stack desync, unrecognised block openers, comment-unaware stripping) all lived in the seams between those stages."
- `BlockStack` (`:243`): "The earlier implementation pushed only named openers while popping on every `}`, so a single `aws = {` desynchronised the stack…"
- `count_local_modules` (`:731`): "the previous `modules/*/main.tf` probe reported … as zero modules".
- The crate-level `//!` block opens with a "Manifest IO policy" section written as ERR-1/TASK-0851 rationale.

Test doc comments carry the same load ("Pre-fix the parser would happily return that string…").

**Why it matters**: a reader of `cargo doc` (or of the source) has to filter a decade of internal ticket numbers to find the one sentence stating current behaviour, and the task IDs are dead references outside this repo's backlog. The rationale is genuinely valuable — it belongs in the commit that made the change and in the test names, which already encode it (`extract_required_version_after_object_valued_required_providers`). This is the Terraform twin of TASK-2191, filed against `extensions-go/about` for the same pattern; `ops-about-rust` and `ops-about-python` are likely candidates for the same treatment.

**Note**: this is a docs-only change — do not weaken any behaviour the comments describe, and keep the non-obvious *current-behaviour* justifications (e.g. why `unicode-ident` rather than `char::is_alphanumeric`, why `fs::metadata` rather than `DirEntry::file_type`, why control chars are dropped rather than stripped) while dropping the "pre-fix it did X" framing and the task IDs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc comments describe current behaviour and the still-live rationale; TASK-NNNN references and 'pre-fix / used to / previously' bug narration are removed
- [ ] #2 Non-obvious design constraints (unicode-ident vs char predicates, fs::metadata vs DirEntry::file_type, drop-not-strip for control chars, deterministic fallback ordering) survive the edit, stated as constraints rather than as incident reports
- [ ] #3 No behavioural change: the existing test suite passes unmodified apart from its own doc comments
- [ ] #4 Approach matches the one taken for TASK-2191 (extensions-go/about)
<!-- AC:END -->
