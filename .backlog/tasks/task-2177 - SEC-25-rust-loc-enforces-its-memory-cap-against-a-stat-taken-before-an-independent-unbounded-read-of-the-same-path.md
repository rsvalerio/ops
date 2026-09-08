---
id: TASK-2177
title: >-
  SEC-25: rust-loc enforces its memory cap against a stat taken before an
  independent, unbounded read of the same path
status: To Do
assignee:
  - TASK-2235
created_date: '2026-09-08 07:12'
updated_date: '2026-09-08 10:54'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/loc/src/lib.rs
priority: low
ordinal: 90000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs:243` (`count_entry`)

**What**: `count_entry` reads the walker's `entry.metadata()` to decide whether a file is under `MAX_SOURCE_BYTES`, then opens the path a second time and calls `std::fs::read_to_string(path)` with no limit at all:

```rust
let size = match entry.metadata() { Ok(m) => m.len(), ... };
let counts = if size > MAX_SOURCE_BYTES { count_streaming(...) }
             else { std::fs::read_to_string(path) ... };
```

The size gate and the read are two independent filesystem operations on a path, not one operation on one handle. Anything that changes the file between them defeats the cap: a generated `.rs` file still being written by a concurrent `cargo expand`, a build script, or a codegen step is appended to after the stat and then read whole. The doc comment on `MAX_SOURCE_BYTES` states the gate exists precisely so that "one machine-written file can[not] OOM the whole `ops` process", and machine-written files are exactly the ones most likely to be mid-write during a scan.

Note the walker uses the default `follow_links(false)` and `count_entry` requires `file_type().is_file()`, so a symlink swap is *not* reachable here — the exposure is the append/replace race, not symlink following.

**Why it matters**: The cap is a resource guard, and a resource guard checked against a stale stat is advisory. The `count_streaming` path exists to bound resident memory for over-cap files; the race routes an over-cap file down the unbounded path instead, and the walk runs on `ignore`'s parallel workers, so the cost is multiplied by the worker count. Low severity because the trigger requires a concurrent writer in the scanned tree, but the fix is small and removes the race entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 count_entry opens the file once and derives the size from that handle (File::open then handle.metadata()), rather than stat-ing the path and re-opening it
- [ ] #2 The in-memory read is itself bounded (e.g. Read::take(MAX_SOURCE_BYTES) into a String), so a file that grows after the size decision cannot be read past the cap
- [ ] #3 A file that turns out to exceed the cap after opening still degrades to the streaming blank-vs-non-blank count rather than being dropped, preserving the documented degradation policy
- [ ] #4 A test covers the over-cap decision being made from the opened handle (an existing over-cap test may be extended rather than adding a race-based test)
<!-- AC:END -->
