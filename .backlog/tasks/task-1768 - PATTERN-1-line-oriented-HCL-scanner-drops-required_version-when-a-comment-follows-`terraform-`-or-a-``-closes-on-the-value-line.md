---
id: TASK-1768
title: >-
  PATTERN-1: line-oriented HCL scanner drops required_version when a comment
  follows `terraform {` or a `}` closes on the value line
status: Triage
assignee: []
created_date: '2026-08-27 11:20'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:264-300` (`block_open_ident`) and `:238-242` (`extract_required_version` tail check)

**What**: The scanner assumes each source line carries exactly one structural token. Two common shapes break that assumption:

1. `block_open_ident` returns `None` unless the trimmed line *ends with* `{` (`:265-267`). `#` and `//` line comments are only skipped when they start the line (`:189`), never stripped from a line tail, so a commented block opener is not recognised:
   ```hcl
   terraform { # pinned for the shared modules
     required_version = ">= 1.5"
   }
   ```
   `terraform` is never pushed, `required_version` is then seen at depth 0, and the whole file yields `None`. Verified against the crate's own `extract_required_version`. (Block comments happen to survive this because `strip_block_comments` blanks them and `line.trim()` removes the residue — the `#` / `//` forms do not.)

2. The value line's tail check (`:238-242`) rejects anything left after the closing quote except a comment, so a same-line close is discarded:
   ```hcl
   terraform {
     required_version = ">= 1.5" }
   ```
   also returns `None`.

**Why it matters**: A trailing comment on a `terraform {` opener is ordinary, human-written HCL — nothing about it is pathological or adversarial — and it silently costs the About card its entire stack-detail line with no `tracing::warn!` to explain the omission. The `strip_inline_comment` helper that already exists at `:366-378` is applied to exactly one call site (the post-quote tail) when the same treatment is needed on every non-string line before structural matching.

**Related**: shares a root cause with the brace-stack desync filed separately against this file — both stem from matching structure per line instead of per token. Fixing them together (one small tokenizer pass, or line-level comment stripping plus brace counting) is likely cheaper than two point patches.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_required_version returns the constraint when the terraform block opener carries a trailing # or // comment
- [ ] #2 A closing brace on the same line as the value (required_version = ">= 1.5" }) still yields the constraint
- [ ] #3 Line comments are stripped outside of double-quoted strings before structural matching, and a # or // inside a quoted value is still preserved
- [ ] #4 Regression tests cover both shapes
<!-- AC:END -->
