---
id: TASK-0148
title: >-
  ERR-5: ensure_config_command silently discards toml_edit parse error with
  unwrap_or_else
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 14:14'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:123-125`

**What**:
```
let mut doc = content
    .parse::<toml_edit::DocumentMut>()
    .unwrap_or_else(|_| toml_edit::DocumentMut::new());
```
If the existing `.ops.toml` has a syntax error, `ensure_config_command` silently discards the parse error *and every byte the user had previously written*, replacing `doc` with an empty document. The subsequent `std::fs::write` then overwrites the user's malformed-but-meaningful file with a near-empty one, causing silent data loss.

**Why it matters**: Critical correctness/data-loss bug. A single typo in `.ops.toml` turns `run-before-commit install` into `rm .ops.toml`. Fix: on parse error, return a descriptive `anyhow::Error` with the parse error attached via `.context()` and the path of the offending file; never fall back to an empty document.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Parse failure returns Err with path + underlying parse error
- [ ] #2 Add test: malformed .ops.toml is not overwritten by install
<!-- AC:END -->
