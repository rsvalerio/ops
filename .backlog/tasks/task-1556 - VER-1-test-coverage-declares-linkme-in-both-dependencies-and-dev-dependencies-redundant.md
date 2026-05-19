---
id: TASK-1556
title: >-
  VER-1: test-coverage declares linkme in both [dependencies] and
  [dev-dependencies] (redundant)
status: To Do
assignee:
  - TASK-1577
created_date: '2026-05-19 15:42'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/Cargo.toml:11,21`

**What**: `linkme = { workspace = true }` is declared in both `[dependencies]` (line 11) and `[dev-dependencies]` (line 21). When a crate is in `[dependencies]` it is automatically available to tests/benches/examples without re-declaration in `[dev-dependencies]`; the duplicate is dead noise that drifts (one may change features and the other lag).

**Why it matters**: Mirrors TASK-1507 in cargo-toml. The redundant declaration confuses dependency audits, can let dev-only features bleed into a prod build if someone later adds features to the dev line, and obscures the real boundary between runtime and dev surface.

<!-- scan confidence: confirmed -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove duplicate linkme entry from [dev-dependencies] in extensions-rust/test-coverage/Cargo.toml
- [ ] #2 cargo build -p ops-test-coverage and cargo test -p ops-test-coverage still pass
<!-- AC:END -->
