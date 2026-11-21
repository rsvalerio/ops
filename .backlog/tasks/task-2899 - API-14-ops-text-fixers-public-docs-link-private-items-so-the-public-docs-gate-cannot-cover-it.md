---
id: TASK-2899
title: 'API-14: ops-text-fixers public docs link private items, so the public-docs gate cannot cover it'
status: Done
assignee: []
created_date: '2026-11-20 21:32'
updated_date: '2026-11-21 07:31'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2901'
modified_files:
  - .ops.toml
  - extensions/text-fixers/src/runner.rs
  - extensions/text-fixers/src/trailing.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/runner.rs:26`, `extensions/text-fixers/src/runner.rs:44`, `extensions/text-fixers/src/trailing.rs:97`

**What**: `cargo rustdoc -p ops-text-fixers --lib --all-features -- -D warnings` fails with `rustdoc::private_intra_doc_links`: `run_trailing_whitespace` and `run_end_of_file_fixer` link the private `run_fixer`, and `fix_trailing` links the private `scan_line`. The new `public-docs` verify gate therefore cannot list ops-text-fixers (ops-config-checkers already passes and can join for free).

**Why it matters**: Published (public-only) docs render dead links for those items; the crate stays outside the regression gate that now covers ops-about, ops-extension and ops-rust-foundation.

**Origin**: discovered during TASK-2897 while fixing TASK-2896.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops-text-fixers passes cargo rustdoc --lib --all-features -- -D warnings without --document-private-items, and ops-text-fixers (and ops-config-checkers) are listed in the public-docs verify gate

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in f8e0346a: run_trailing_whitespace/run_end_of_file_fixer now point at FixerReport::files_failed and FixerReport::failed instead of the private run_fixer; fix_trailing no longer links scan_line. Added public-docs-text-fixers and public-docs-config-checkers to the public-docs gate. rustdoc -D warnings: text-fixers 101 before, 0 after. Pre-merge and integration ops verify 16/16; nextest text-fixers+config-checkers 127/127.
<!-- SECTION:NOTES:END -->
