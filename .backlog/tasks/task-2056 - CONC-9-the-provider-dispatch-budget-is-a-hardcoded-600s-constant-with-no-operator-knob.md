---
id: TASK-2056
title: >-
  CONC-9: the provider dispatch budget is a hardcoded 600s constant with no
  operator knob
status: To Do
assignee:
  - TASK-2060
created_date: '2026-08-29 13:31'
updated_date: '2026-08-29 17:27'
labels:
  - code-review-rust
  - concurrency
dependencies: []
modified_files:
  - crates/extension/src/data.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs` (`DEFAULT_PROVIDER_BUDGET`)

**What**: TASK-2017 gave provider dispatch a wall-clock bound, but the value is
a `const` compiled into `ops-extension` and the only override is the
programmatic `Context::with_provider_budget`, which nothing in the CLI wiring
calls. An operator on a slow network filesystem cannot tighten it, and an
operator running a genuinely long provider (a full-workspace `cargo llvm-cov`
through the coverage providers) cannot widen it or turn it off; the constant
has to be generous enough for the slowest in-tree provider, which makes it
loose for the ones the SEC-33 finding was actually about.

This is the same shape as TASK-2022 (the post-exit capture drain deadline),
which was filed and fixed for exactly this reason.

**Why it matters**: a timeout nobody can configure is tuned for the worst case
and therefore protects the common case badly. It also means the only way to
diagnose a suspected provider stall is to rebuild with a different constant.

**Origin**: discovered during TASK-2047 while fixing TASK-2017.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The provider dispatch budget is readable from config (with the current constant as the default) rather than only from a const
- [ ] #2 The wiring that builds a Context applies the configured value, pinned by a test
<!-- AC:END -->
