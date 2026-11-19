---
id: TASK-2680
title: 'DUP-1: capped read-then-decode logic duplicated between config loader and text::read_capped_to_string_with'
status: Done
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 22:14'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2841'
modified_files:
  - crates/core/src/config/loader/mod.rs
  - crates/core/src/text.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-1:crates/core/src/config/loader/mod.rs:read_capped_toml_file_with_policy'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader/mod.rs:132-191` (`read_capped_toml_file_with_policy`), `crates/core/src/text.rs:472-508` (`read_capped_to_string_with`)

**What**: Both functions open through `open_refusing_symlinks`, read `take(cap.saturating_add(1)).read_to_end(&mut bytes)`, convert `bytes.len()` with `u64::try_from(..).unwrap_or(u64::MAX)`, compare against `cap`, then `String::from_utf8`. The comments cross-reference each other ("same ordering as `text::read_capped_to_string_with`", "Shared with ... so the two read_capped_* entry points cannot diverge again") — the divergence already happened once (the UTF-8-before-size-check bug was fixed in both places separately). Only the error type (io::Error vs anyhow with the override env var name) and the symlink policy differ.

**Why it matters**: A future change to the cap semantics (e.g. reading with `metadata().len()` pre-sizing, or a new refusal class) has to be made twice; the comments show the two copies have drifted before.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One helper performs open (with a symlink-policy parameter) + capped byte read + size check + UTF-8 decode, returning a typed error that distinguishes NotFound, oversize and invalid UTF-8
- [x] #2 read_capped_toml_file_with_policy and read_capped_to_string_with both delegate to it and keep their current error messages (path, cap, override env var)
- [x] #3 Existing cap and UTF-8-boundary tests for both entry points pass unchanged

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.03 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
