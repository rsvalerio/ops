---
id: TASK-1922
title: >-
  SEC-33: collect_rust_loc has no byte cap on the files it reads, and
  count_source holds ~10x each file's size resident while parsing it twice
status: Done
assignee:
  - TASK-1998
created_date: '2026-08-27 15:45'
updated_date: '2026-08-28 15:36'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/loc/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs:135` (`collect_rust_loc`)

**What**: The walk reads every `.rs` file it finds with `std::fs::read_to_string(path)` and passes the whole string to `count_source`, with no size gate anywhere on the path. For one file, the peak resident set is roughly:

- the owned `String` from `read_to_string`
- `Vec<&str>` of line slices (counter.rs:188)
- `Vec<LineKind>` and `Vec<Region>`, one entry per line (counter.rs:198, 209)
- the full `proc_macro2::TokenStream` from `TokenStream::from_str` (counter.rs:194)
- the `syn::File` AST from `syn::parse_file` (counter.rs:211), which re-lexes the source from scratch rather than reusing the stream already in hand
- proc-macro2's `span-locations` source map, which retains an owned copy of the source plus a line table

The `invalidate_current_thread_spans()` call at counter.rs:186 bounds the retention across files, but does nothing for a single large one. On top of that, every emitted row accumulates into one `Vec<serde_json::Value>` (lib.rs:113) returned as a single `serde_json::Value::Array`, which the ingestor then serializes whole into `rust_loc_files.json`.

Nothing in the tree is off limits: a vendored bindgen output, a generated parser table, a concatenated build artifact, or any large machine-written `.rs` file is read and parsed at full size.

**Why it matters**: `collect_rust_loc` already has a well-designed degradation policy for unreadable files and unwalkable subtrees - warn and skip, because "a partial count is more useful than none" (lib.rs:99-107). Oversized input is the one resource failure that policy does not cover, so instead of a warned skip it produces an OOM kill of the whole `ops` process with no log line explaining why. The cheap fix already exists in the crate: `count_fallback` counts blank vs non-blank lines with no parsing and no AST, so an oversized file can still contribute an honest line count.

Consistent with TASK-1866 (no byte cap on the .git/HEAD read) and TASK-1811 (unbounded fs::read behind an advisory size gate).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A named constant defines the maximum byte size of a .rs file that collect_rust_loc will lex and parse
- [x] #2 Files over the cap are counted with count_fallback (or skipped) and emit a tracing::warn that names the path with the Debug formatting already used at lib.rs:141 and states the cap, so a degraded count is never silent
- [x] #3 The size is checked before the file contents are pulled fully into memory, using the walker's DirEntry metadata rather than reading and then measuring
- [x] #4 A test writes an over-cap .rs file into a tempdir alongside a normal one and asserts the normal file's rows are still emitted and the over-cap file does not abort the scan
<!-- AC:END -->
