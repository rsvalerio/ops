---
id: TASK-2379
title: 'ERR-4: check_member_opt_in swallows a member manifest parse error as a misleading drift'
status: Triage
assignee: []
created_date: '2026-10-04 14:13'
labels:
  - code-review-rust
  - ERR
dependencies: []
modified_files:
  - extensions-rust/foundation/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-4:extensions-rust/foundation/src/lib.rs:check_member_opt_in'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs:405` (`check_member_opt_in`)

**What**: `toml::from_str(&text).ok().and_then(...).unwrap_or(false)` turns a member `Cargo.toml` that does not parse into "missing `[lints] workspace = true`". The parse error is discarded. `scaffold_member_opt_in` handles the same input differently: it returns an error with the parse context. `check_file` reports parse failures as their own drift.

**Why it matters**: The reported fix is wrong, because adding `[lints] workspace = true` to a broken manifest does not help. The error also never names the real problem, and `scaffold` and `check` disagree about the same member.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An unparseable member manifest produces a distinct 'does not parse' drift (or error) that includes the path and the parse message, not the missing-opt-in message
- [ ] #2 A test covers a member with malformed TOML for both check and scaffold
<!-- AC:END -->
