---
id: TASK-1931
title: >-
  PERF-10: the whole-workspace .rs scan is fully serial — one thread reads,
  lexes and syn-parses every file in sequence while ignore's parallel walker
  sits unused
status: To Do
assignee:
  - TASK-1998
created_date: '2026-08-27 15:46'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - extensions-rust/loc/src/lib.rs
  - extensions-rust/loc/src/counter.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs:115` (`collect_rust_loc`)

**What**: `WalkBuilder::new(working_dir).filter_entry(..).build()` returns the single-threaded `Walk` iterator, and the `for entry in walker` loop does all the work inline on one thread: `read_to_string`, then `count_source`, which lexes the source with `TokenStream::from_str` and then lexes it a **second** time inside `syn::parse_file` (counter.rs:194 and 211 - `syn::parse_file` re-lexes from scratch rather than reusing the `TokenStream` already in hand). Every `.rs` file in the workspace pays both passes, one after another.

Two things make this straightforward to fix, and both are already true today:

- `ignore` is already a direct dependency and `WalkBuilder::build_parallel()` is the parallel counterpart of the walker already in use, with `filter_entry` supported identically.
- `count_source` is thread-safe and per-thread independent by construction. proc-macro2's `span-locations` source map is a thread-local, and `invalidate_current_thread_spans()` (counter.rs:186) only touches the calling thread's map - the property `repeated_counts_on_one_thread_are_independent` already pins. Each worker thread would simply keep its own map, which is strictly better than today's single shared one.

The second, independent lex is avoidable on its own: `syn::parse2::<syn::File>(stream)` accepts the `TokenStream` that line 194 already produced. Caveat for whoever implements it - `syn::parse_file` strips a leading `#!` shebang and `parse2` does not, so that case needs handling or an explicit note.

**Why it matters**: This runs over every Rust file in the repository on every `ops` data collection, and it is pure CPU-bound parsing work with no shared mutable state - the textbook case for the parallel walker. PERF-6 applies: this is a candidate, not a proven hotspot, so the fix should be measured rather than assumed. The measurement is cheap because the extension already has a `#[ignore]`d whole-crate scan test (`collect_rust_loc_returns_records_for_this_crate`) that can be pointed at the workspace root.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A before/after wall-clock measurement of collect_rust_loc over the ops workspace root is recorded in the task before any change is kept, per PERF-6
- [ ] #2 If the measurement justifies it, collect_rust_loc uses WalkBuilder::build_parallel with the same filter_entry pruning, collecting rows through a shared sink; record ordering must not be relied on by any test or by the DuckDB ingest
- [ ] #3 The redundant second lex is removed by handing the existing TokenStream to syn::parse2, with the shebang difference versus syn::parse_file either handled or documented in counter.rs
- [ ] #4 Existing tests still pass unchanged, including repeated_counts_on_one_thread_are_independent and the collect_rust_loc tempdir tests
- [ ] #5 If the measurement shows no meaningful gain, the task is closed with the numbers recorded and the serial walk is left in place
<!-- AC:END -->
