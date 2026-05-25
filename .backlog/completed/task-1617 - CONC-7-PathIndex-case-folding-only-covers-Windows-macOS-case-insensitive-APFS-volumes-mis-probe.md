---
id: TASK-1617
title: >-
  CONC-7: PathIndex case-folding only covers Windows; macOS case-insensitive
  APFS volumes mis-probe
status: Done
assignee:
  - TASK-1637
created_date: '2026-05-22 06:52'
updated_date: '2026-05-22 12:56'
labels:
  - code-review-rust
  - correctness
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/path.rs:21-27` (`index_key`), used by `is_in_path_index` at `path.rs:69-83`.

**What**: `index_key` lowercases the basename only under `cfg!(windows)`. macOS APFS volumes default to case-*insensitive* (case-preserving), and HFS+ has always been case-insensitive by default. On those filesystems a tool installed as `Tokei` will be visited by `read_dir` with its on-disk casing, indexed verbatim, and then `is_in_path_index(&idx, "tokei")` will miss — the probe reports `NotInstalled` and the install path will then attempt to re-install a tool that genuinely lives on `$PATH`. This is the same regression CONC-7 / TASK-1249 already fixed for Windows; macOS shares the property.

**Why it matters**: The dev population for this crate skews heavily macOS. The existing CONC-7 / TASK-1249 fix and its test (`path_index_case_tests`) document the contract one OS at a time; the macOS gap leaves the contract two-thirds finished. The downstream effect — spurious `cargo install` re-runs — is the precise failure mode API / TASK-1200 went out of its way to prevent for probe failures.

<!-- scan confidence: high; behavioural divergence reproduced by reading `cfg!(windows)` branches against APFS defaults -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Detect case-insensitive filesystems on macOS (or unconditionally lowercase under cfg(target_os = "macos")) so index_key normalises consistently with the FS the lookup is performed against.
- [x] #2 Add a #[cfg(target_os = "macos")] test mirroring path_index_case_tests::windows_lookup_matches_mixed_case_basename to lock the contract in.
<!-- AC:END -->
