---
id: TASK-1779
title: >-
  FN-1: extract_required_version spans 93 lines mixing comment stripping, brace
  tracking, key matching, quote parsing and capping
status: Triage
assignee: []
created_date: '2026-08-27 11:22'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:167-259` (`extract_required_version`), also `:89-150` (`find_required_version`, 62 lines)

**What**: `extract_required_version` is a single 93-line function that performs five distinct jobs in one loop body: block-comment pre-pass, line-comment skipping, block-stack push/pop, `required_version = "…"` shape matching (five sequential `strip_prefix`/`split_once` guards with `let-else` continues), and the SEC-11 length cap with its warn. Nesting reaches four levels and the loop carries mutable state (`block_stack`) that every branch can affect. `find_required_version` is a second over-threshold function in the same file, combining a candidate-list probe, a `read_dir` fallback with sort, and per-file extraction.

**Why it matters**: This is not a style nit here — three separate correctness bugs found in this review (brace-stack desync on object-valued attributes, unrecognised block openers with trailing comments, line-comment-unaware block stripping) all live in the seams between those five responsibilities, and none of them is visible when reading any one stanza in isolation. A structure of `strip_comments(content) -> String`, `hcl_block_path(line, &mut stack)`, and `parse_required_version_assignment(line) -> Option<&str>` would make each rule independently testable and would have made the desync obvious. Splitting also gives the fixes for the three bugs a natural landing site instead of three more guards inside the same loop.

**Note**: severity is low as a standalone refactor; sequence it after (or together with) the correctness fixes filed against this file so the tests written for those land against the new shape rather than being rewritten twice.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_required_version is decomposed so comment stripping, block-path tracking and assignment parsing are separate, individually testable units
- [ ] #2 No function in the file exceeds the 50-line FN-1 threshold
- [ ] #3 Existing behaviour and tests are preserved; cargo test -p ops-about-terraform passes
<!-- AC:END -->
