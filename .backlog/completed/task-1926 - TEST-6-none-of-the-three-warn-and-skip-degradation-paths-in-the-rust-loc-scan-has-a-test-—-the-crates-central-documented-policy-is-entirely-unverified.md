---
id: TASK-1926
title: >-
  TEST-6: none of the three warn-and-skip degradation paths in the rust-loc scan
  has a test — the crate's central documented policy is entirely unverified
status: Done
assignee:
  - TASK-1998
created_date: '2026-08-27 15:45'
updated_date: '2026-08-28 15:37'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/loc/src/tests.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/tests.rs` (missing coverage for `extensions-rust/loc/src/lib.rs:119-144` and `extensions-rust/loc/src/counter.rs:210-214`)

**What**: The rust-loc extension's defining behaviour is that it degrades instead of failing. Three separate degradation branches implement that, and none of them is exercised by any test in `src/tests.rs`:

1. **Unwalkable path** (lib.rs:120-126) - `Err(error) => { tracing::warn!(...); continue; }` on a walker entry.
2. **Unreadable file** (lib.rs:135-144) - `Err(error) => { tracing::warn!(...); continue; }` on `read_to_string`.
3. **Lexes but does not parse** (counter.rs:210-214) - `if let Ok(file) = syn::parse_file(src)` silently drops the whole test-attribution pass when `syn` rejects a file that `proc_macro2` accepted, so every line is attributed to `base`. `unlexable_source_falls_back_to_blank_versus_nonblank` covers the `TokenStream::from_str` failure at counter.rs:194, but not this one - they are different branches with different outcomes.

Branch 3 is trivially reachable and worth pinning: `let x = 1;` is a valid token stream but not a valid top-level item, so `TokenStream::from_str` succeeds and `syn::parse_file` fails.

Branch 2 has a deterministic, permission-free, root-safe reproduction: write a `.rs` file containing invalid UTF-8 bytes. `read_to_string` fails with `InvalidData` regardless of who runs the suite, which avoids the `chmod 0o000`-under-root trap already filed as TASK-1802.

**Why it matters**: TEST-6 asks for error paths and edge cases, and these are not incidental - they are the reason the crate's module docs spend two paragraphs (lib.rs:99-107, counter.rs:165-170) explaining the design. A regression that turns any `continue` into an early `return`, or that makes a read failure abort the walk, produces a silently short line count: the About page still renders, the numbers are just wrong, and no test fails. The counts have no external oracle, so a test is the only thing that can catch it.

This also protects the invariant behind TASK-1924 (the # Errors doc contradicting the skip policy): whichever way that is resolved, a test should pin it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A test writes a .rs file with invalid UTF-8 bytes into a tempdir next to a valid one, calls collect_rust_loc, and asserts the valid file's rows are still returned and the scan does not error
- [x] #2 A test covers the walker-error branch at lib.rs:120-126, for example via an unreadable subdirectory, and skips itself with a clear message when the process can read it anyway (running as root) rather than passing vacuously
- [x] #3 A test passes a source that lexes but fails syn::parse_file, such as a bare 'let x = 1;' at file scope, and asserts the lines are counted and all attributed to the base region with no test split
- [x] #4 Each new test names the branch it pins in its test name or doc comment, so the link between the degradation policy and its coverage is visible
<!-- AC:END -->
