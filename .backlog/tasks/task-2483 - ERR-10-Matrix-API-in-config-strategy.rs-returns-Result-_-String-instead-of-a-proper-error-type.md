---
id: TASK-2483
title: 'ERR-10: Matrix API in config/strategy.rs returns Result<_, String> instead of a proper error type'
status: To Do
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2610'
modified_files:
  - crates/core/src/config/strategy.rs
priority: medium
ordinal: 1000
dedup_key: 'ERR-10:crates/core/src/config/strategy.rs:Matrix'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/strategy.rs:132,177,297,310,319` (`Matrix::cells`, `Matrix::product`, `check_key`, `check_value`, `check_entry`)

**What**: Five functions in the matrix-slicing feature return stringly-typed errors: `pub fn cells(&self) -> Result<Vec<MatrixCell>, String>` (line 132) plus the private `product()` (177), `check_key()` (297), `check_value()` (310), and `check_entry()` (319), all building error text with `format!`/`to_string()`. The same file already defines the typed pattern to follow: `MatrixRefError` (used by the `FromStr`/reference-resolution side) carries structured context. `cells()` is the public entry point; its only in-crate caller immediately wraps the `String` into `anyhow` via `.map_err`, so the error loses type information at the boundary and downstream code cannot match on the failure kind.

**Why it matters**: ERR-10: never use `Result<T, String>` — use proper error types. A `String` cannot be matched, carries no source chain, and pushes message-formatting decisions into every intermediate layer. Reusing or extending `MatrixRefError` (or adding a sibling variant set for key/value/entry validation) keeps the public surface self-describing and lets callers classify errors without string matching.

<!-- scan confidence: verified by direct read -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No function in config/strategy.rs returns Result<_, String>; the public cells() API and the private validators return a typed error (MatrixRefError or a sibling) carrying the key/value/entry context
- [ ] #2 The in-crate caller of cells() propagates the typed error instead of re-wrapping a String via map_err
<!-- AC:END -->
