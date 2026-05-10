---
id: TASK-0941
title: >-
  ERR-7: extensions-rust about/query.rs glob walk warn events use Display for
  parent/pattern/error
status: Done
assignee: []
created_date: '2026-05-02 16:02'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:267-282`

**What**: The two `tracing::warn!` events covering unreadable workspace glob entries log `parent = %parent.display()`, `pattern = %member`, `error = %e` via Display. The `member` value comes from `[workspace].members` in a Cargo.toml that may be attacker-controlled (cloned repo).

**Why it matters**: The TASK-0930 sweep covered the Node-side equivalent in extensions-node/about. This Rust-side site is the parallel gap. Embedded newlines/ANSI can forge log lines or hide diagnostics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Switch parent, pattern, error fields to Debug formatter at both warn sites (lines 267-271 and 277-282)
- [x] #2 Regression test mirrors the package_json_path_debug_escapes_control_characters pattern, asserting embedded \n/\u{1b} are escaped
<!-- AC:END -->
