---
id: TASK-1468
title: 'SEC-25: read_capped_toml_file_with does not refuse to follow symlinks'
status: Done
assignee:
  - TASK-1478
created_date: '2026-05-16 10:05'
updated_date: '2026-05-17 07:18'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:78-99`

**What**: `read_capped_toml_file_with` opens `path` with `std::fs::File::open` directly, with no `symlink_metadata` probe — unlike its sibling `text.rs::read_capped_to_string_with` (which explicitly rejects symlinks per SEC-25 / TASK-1442).

**Why it matters**: `ops` is invoked inside third-party repos; a planted `.ops.toml -> /etc/shadow` (or any user-readable secret) will be slurped into the TOML parser, and the parse-error path echoes file contents back through `with_context`/diagnostics — the same information-disclosure shape TASK-1442 fixed for manifest reads.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a symlink_metadata-based reject in read_capped_toml_file_with (mirroring text.rs:142-156) before File::open, returning Err with ErrorKind::InvalidInput
- [ ] #2 Add a #[cfg(unix)] test that plants .ops.toml -> /etc/passwd and asserts the loader rejects it without parsing or echoing the target
- [ ] #3 Factor the symlink-reject probe into a shared helper so the two read_capped_* paths cannot diverge again
<!-- AC:END -->
