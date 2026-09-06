---
id: TASK-1970
title: >-
  SEC-33: collect_tokei walks and reads an arbitrary tree with no file-size,
  record-count or depth cap
status: Done
assignee:
  - TASK-2012
created_date: '2026-08-27 15:54'
updated_date: '2026-08-28 15:59'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/tokei/src/lib.rs
  - extensions/tokei/src/ingestor.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:123-143` (`collect_tokei`, `flatten_tokei_to_json`), reached from `extensions/tokei/src/ingestor.rs:19-22`

**What**: `collect_tokei` hands `ctx.working_directory` straight to `Languages::get_statistics` and then materialises one JSON object per discovered file. Nothing anywhere on the path bounds the work:

- **No per-file byte cap.** tokei 14.0 reads each candidate file whole into memory to count its lines (tokei-14.0.0/src/utils/fs.rs walks with `ignore::WalkBuilder`, then parses each entry). A single multi-GB file that happens to carry a recognised extension -- a checked-in database dump named `.sql`, a generated `.json`, a vendored `.c` blob -- is read entirely into RAM. This is the same defect filed for the sibling LOC counter as TASK-1922, but here the read happens inside the tokei dependency, so the cap has to be applied before the walk (skip by `metadata().len()`), not inside a read loop we own.
- **No record-count cap.** `flatten_tokei_to_json` collects every report of every language into an unbounded `Vec<serde_json::Value>` (lines 133-141), returns it whole, and `SidecarIngestorConfig::collect_sidecar` then serialises the entire array to `tokei_files.json`. Peak memory is the full per-file record set plus its serialised form, held simultaneously. On a monorepo with hundreds of thousands of files this is hundreds of MB for a statistic nobody asked to be exact.
- **No walk-depth cap.** `ignore::WalkBuilder` defaults to `max_depth(None)`. `follow_links` defaults to false so a symlink cycle will not loop, but a genuinely deep or generated tree is walked to the bottom.
- **No timeout.** `DataProvider::provide` is synchronous and runs the whole scan inline on every call when no DuckDB handle is present (`extensions/duckdb/src/lib.rs:57-61` falls through to the closure at `extensions/tokei/src/lib.rs:60-62`), so a slow or hostile tree stalls the CLI with no upper bound and no way to interrupt it.

The input is not notionally trusted: `working_directory` is whatever directory the user points ops at, and the tree under it is arbitrary third-party content (dependencies, generated output, artefacts). The hardcoded exclusion list at lines 110-118 removes the usual build directories but does nothing about a large file in `src/`.

**Why it matters**: SEC-33 -- an LOC statistic is a cosmetic feature that can currently take the whole `ops` process down by OOM, or hang it indefinitely, on input the user did not author. Degrading (skip the oversized file, record it as skipped, stop after N records) costs nothing here because the output is advisory.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A per-file byte cap is enforced before tokei parses a file; files over the cap are skipped rather than read whole
- [x] #2 The number of per-file records materialised by flatten_tokei_to_json is bounded, and a truncated result is reported rather than silently returned as complete
- [x] #3 The walk has an explicit maximum depth
- [x] #4 A test builds a fixture exceeding the byte cap and asserts collect_tokei completes without reading it whole, and a test asserts the record cap truncates rather than allocating without bound
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2012 (branch code-review/TASK-2012).

`collect_tokei` no longer hands the tree to tokei's walker. It now walks with
`ignore::WalkBuilder` itself under an explicit `ScanLimits` budget and passes
tokei only the candidate file list:

- AC #1: `ScanLimits::file_bytes` (4 MiB default) is checked from the walk
  entry's metadata, so an oversized file is never opened, let alone read whole.
- AC #2: `ScanLimits::files` (50k default) bounds the candidate list and
  therefore the records materialised; hitting it sets `TokeiScan::truncated`,
  which `collect_tokei` reports through a warning instead of returning a
  truncated array as if it were complete.
- AC #3: `ScanLimits::depth` (32) is passed to `WalkBuilder::max_depth`.
- AC #4: `scan_tokei_skips_files_over_the_byte_cap`,
  `scan_tokei_truncates_at_the_file_cap` and `scan_tokei_honours_the_depth_cap`
  drive `scan_tokei` with lowered limits, so the fixtures stay small and the
  assertions are exact.

Not addressed: the finding's fourth bullet (no timeout on the synchronous
`provide` path) is a `DataProvider` trait-level concern, not something this
crate can fix alone. The size, count and depth caps bound the work that made an
unbounded stall reachable here.

Timeout bullet discharged: filed TASK-2017 (Triage) against the DataProvider trait, where a dispatch-level bound belongs.
<!-- SECTION:NOTES:END -->
