---
id: TASK-2385
title: 'READ-10: Bare allow(clippy::option_if_let_else) without reason in strip_one_line'
status: Triage
assignee: []
created_date: '2026-10-04 14:13'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-10:extensions-terraform/about/src/lib.rs:strip_one_line'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:875`

**What**: `#[allow(clippy::option_if_let_else)]` on a local `let` in `strip_one_line` carries a rationale only in a preceding `//` comment, not in the attribute. It is a per-site suppression of a lint the code could stop tripping, not a permanent policy exception.

**Why it matters**: `#[allow]` outlives the problem; `#[expect(..., reason = "...")]` (stable since 1.81, workspace rust-version is 1.97) warns with `unfulfilled_lint_expectations` once the lint stops firing and keeps the reason where tooling can see it. Restructuring the split (strip_suffix then unwrap_or(line), newline taken from the remainder) removes the need for the suppression entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The suppression is removed by restructuring the split, or converted to expect with a reason
- [ ] #2 cargo clippy --workspace --all-targets reports no new warnings
<!-- AC:END -->
