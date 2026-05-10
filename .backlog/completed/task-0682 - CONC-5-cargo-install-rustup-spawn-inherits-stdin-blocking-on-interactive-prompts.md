---
id: TASK-0682
title: >-
  CONC-5: cargo install/rustup spawn inherits stdin, blocking on interactive
  prompts
status: Done
assignee:
  - TASK-0735
created_date: '2026-04-30 05:15'
updated_date: '2026-04-30 06:13'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:63-65, 86-88`

**What**: `cargo install` / `rustup component add` spawn with default stdio (inherits stdin from `ops`).

**Why it matters**: In CI or piped contexts, an interactive prompt (rustup occasionally asks for confirmation in unusual repos) blocks until timeout. Closing stdin makes the child hit EOF and bail deterministically.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Set stdin(Stdio::null()) on both spawn calls
<!-- AC:END -->
