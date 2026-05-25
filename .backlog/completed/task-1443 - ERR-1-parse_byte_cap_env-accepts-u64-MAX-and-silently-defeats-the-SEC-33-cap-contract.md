---
id: TASK-1443
title: >-
  ERR-1: parse_byte_cap_env accepts u64::MAX and silently defeats the SEC-33 cap
  contract
status: Done
assignee:
  - TASK-1454
created_date: '2026-05-13 18:44'
updated_date: '2026-05-13 21:48'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:38-61`

**What**: `parse_byte_cap_env` accepts any positive `u64` including `u64::MAX`. Combined with `read_capped_to_string_with`'s `cap.saturating_add(1)` and `Read::take(limit)`, a misconfigured `OPS_MANIFEST_MAX_BYTES=18446744073709551615` defeats the SEC-33 contract entirely (effectively "no cap") with no warn — the helper's whole reason for existing.

**Why it matters**: Mirrors the `MAX_TIMEOUT_SECS` clamp pattern already adopted in `subprocess.rs::parse_subprocess_timeout`. Without a sane upper bound the cap silently degrades to "unlimited" — the inverse of what operators set the variable to control.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_byte_cap_env clamps to a documented upper bound (e.g. 1 GiB) and emits a one-shot warn when clamped
- [x] #2 Unit test for u64::MAX input clamps and returns/logs a warn message
- [x] #3 Cross-reference to subprocess::parse_subprocess_timeout in the doc comment
<!-- AC:END -->
