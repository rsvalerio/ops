---
id: TASK-1442
title: >-
  SEC-25: read_capped_to_string follows symlinks on manifest paths, allowing
  info-leak via crafted repo
status: Done
assignee:
  - TASK-1450
created_date: '2026-05-13 18:44'
updated_date: '2026-05-13 19:14'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:117-129`

**What**: `read_capped_to_string_with` calls `std::fs::File::open(path)` on caller-supplied manifest paths (`Cargo.toml`, `go.mod`, `gradle.properties`, etc.) probed under user-controlled CWDs. There is no symlink protection: an adversarial repo can plant `package.json -> /etc/passwd` (or any privileged file the invoking user can read) and downstream renderers / diagnostics will surface the contents.

**Why it matters**: `ops` is invoked on third-party repos. The SEC-33 byte cap blunts OOM but not info-leak. An `O_NOFOLLOW` open (or pre-`symlink_metadata` check that bails on link kind) closes the gap. Mirrors the SEC-25 class of "atomic_write follows symlinks" finding (TASK-1388) on the read side.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On Unix, read_capped_to_string opens with O_NOFOLLOW (or symlink_metadata rejects link kinds before File::open)
- [ ] #2 Unit test plants a symlink at a manifest path and asserts a typed 'refusing to follow symlink' error
- [ ] #3 Behaviour for regular files and missing files is unchanged
<!-- AC:END -->
