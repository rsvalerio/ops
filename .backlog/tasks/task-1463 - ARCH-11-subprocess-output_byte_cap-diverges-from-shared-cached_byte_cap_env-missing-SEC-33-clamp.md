---
id: TASK-1463
title: >-
  ARCH-11: subprocess::output_byte_cap diverges from shared cached_byte_cap_env,
  missing SEC-33 clamp
status: To Do
assignee:
  - TASK-1479
created_date: '2026-05-15 18:50'
updated_date: '2026-05-17 07:06'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:198-215` vs `crates/core/src/text.rs:99-108`

**What**: `text.rs` factored `cached_byte_cap_env` so all byte-cap env knobs share warn/clamp/fallback semantics, including the `BYTE_CAP_ENV_MAX` clamp. `subprocess.rs::output_byte_cap` is a near-identical hand-rolled `OnceLock<usize>` that lacks the upper-bound clamp. `OPS_OUTPUT_BYTE_CAP=18446744073709551615` (or any large value) silently disables the SEC-33 cap for subprocess capture — exactly the failure mode the shared helper was built to prevent elsewhere.

**Why it matters**: A SEC-33 regression: an attacker (or a misconfigured CI env) can disable the in-memory output cap on cargo subprocess capture by setting an arbitrarily large `OPS_OUTPUT_BYTE_CAP`, leading to OOM on a runaway child. The shape divergence also guarantees the two paths drift further on the next change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 subprocess::output_byte_cap routes through cached_byte_cap_env (or a usize sibling) so it inherits the upper-bound clamp and warn shape
- [ ] #2 Regression test sets OPS_OUTPUT_BYTE_CAP=u64::MAX and asserts the resolved cap is BYTE_CAP_ENV_MAX rather than the requested value
<!-- AC:END -->
