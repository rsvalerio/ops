---
id: TASK-0681
title: >-
  ERR-1: cargo-update strip_ansi only consumes SGR (m) terminators, dropping
  other CSI shapes
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:15'
updated_date: '2026-04-30 18:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:151-168`

**What**: Loops until `m`, so any other CSI final byte (`K`, `J`, `H`, `;` queries…) leaves stray bytes that break downstream `starts_with("Updating")` checks.

**Why it matters**: cargo-update progress lines use \x1b[2K (erase line) interleaved with SGR; if cargo-edit ever switches a verb line through \x1b[K the parser will drop the line entirely and produce silent count regressions — exactly the failure mode TASK-0472 added a warn for, but here the warn won't fire because the verb prefix isn't visible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace with a CSI-aware stripper (consume [, params, then any 0x40..=0x7E final byte) or use the strip-ansi-escapes crate
<!-- AC:END -->
