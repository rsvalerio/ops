---
id: TASK-2765
title: 'ARCH-1: Split the HCL scanner out of ops-about-terraform''s 1,100-line lib.rs'
status: To Do
assignee: []
created_date: '2026-11-18 18:42'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - ARCH
dependencies: []
parent_task_id: 'TASK-2856'
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'ARCH-1:extensions-terraform/about/src/lib.rs:lib.rs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:1` (production code spans lines 1-1108; tests 1110-2268)

**What**: `lib.rs` holds about 1,100 lines of production code and mixes three concerns that have no reason to live together:
1. Extension and provider wiring (`AboutTerraformExtension`, `TerraformIdentityProvider`, lines 45-97).
2. Filesystem discovery (`find_required_version`, `fallback_tf_paths`, `read_dir_logged`, `count_local_modules`, `is_module_dir`, `contains_terraform_source`, lines 99-205 and 1039-1108).
3. A hand-written HCL lexer of roughly 800 lines: `strip_comments`, `strip_one_line`, `strip_code_chars`, `push_heredoc_opener`, `blank_*_comment*`, `StripState`/`StripEof`, `scan_required_version`, `scan_line`, `scan_code_chars`, `LineWalk`, `ScanState`, `HeredocTracker`, `Heredoc`, `heredoc_terminator`, `block_open_ident`, `parse_required_version_assignment`, `sanitize_required_version`, and `Refusal` (lines 207-1037).

The 1,150-line `#[cfg(test)] mod tests` covers all three in one block as well.

**Why it matters**: ARCH-1 treats a module over 500 lines with mixed concerns as a red flag, and ARCH-8 expects `lib.rs` to be a thin entry point. The lexer is the subtle, correctness-critical part of the crate: its two stages (comment stripping, then structural scan) must agree on string, heredoc and comment boundaries. Isolated in its own module, its invariants and tests can be read without wading through extension wiring and directory walking, and its pure `&str -> Result<Option<String>, Refusal>` surface becomes the module boundary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The HCL comment-stripping and required_version scanning code (strip_*, scan_*, heredoc tracking, Refusal, sanitize) lives in its own module (e.g. src/hcl_scan.rs) with its unit tests beside it
- [ ] #2 Filesystem discovery (required-version file walk, local module counting) is separated from extension wiring, leaving lib.rs as a thin entry point
- [ ] #3 No behaviour change: the existing test suite passes unchanged apart from moves, and ops verify is green
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.20 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
