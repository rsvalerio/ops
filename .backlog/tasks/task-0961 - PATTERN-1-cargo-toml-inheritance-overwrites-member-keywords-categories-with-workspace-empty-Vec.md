---
id: TASK-0961
title: >-
  PATTERN-1: cargo-toml inheritance overwrites member keywords/categories with
  workspace empty Vec
status: Triage
assignee: []
created_date: '2026-05-04 21:47'
labels:
  - code-review-rust
  - correctness
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:46-80`

**What**: `resolve_package_inheritance` resolves `keywords` and `categories` via `resolve_vec_field`, but `WorkspacePackage`'s corresponding fields are plain `Vec<String>` (not `Option<Vec<String>>`), so an absent workspace `keywords` table is indistinguishable from an empty list and is propagated as an empty Vec to the package. Member crates that opt into `keywords = { workspace = true }` get a forced empty Vec when the workspace did not declare keywords, instead of leaving the field unset.

**Why it matters**: Silent override of member intent. The "inherit only if defined" contract is violated for keywords/categories.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Make WorkspacePackage.keywords/categories Option<Vec<String>> (or guard inside resolve_vec_field against ws empty), so absence stays distinguishable from explicit empty
- [ ] #2 Test asserts keywords inherit-from-workspace with no workspace keywords leaves the field as Inherited, not as a value-bearing empty Vec
<!-- AC:END -->
