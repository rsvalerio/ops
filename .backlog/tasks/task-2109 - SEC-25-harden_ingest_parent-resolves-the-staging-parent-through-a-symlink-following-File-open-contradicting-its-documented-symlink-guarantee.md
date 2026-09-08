---
id: TASK-2109
title: >-
  SEC-25: harden_ingest_parent resolves the staging parent through a
  symlink-following File::open, contradicting its documented symlink guarantee
status: To Do
assignee:
  - TASK-2235
created_date: '2026-09-08 06:53'
updated_date: '2026-09-08 10:53'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/duckdb/src/sql/ingest/dir.rs
priority: medium
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest/dir.rs:196`

**What**: `harden_ingest_parent` opens the ingest staging parent with
`std::fs::File::open(parent)?`, which follows symlinks, and then decides the
directory is safe on `meta.is_dir()`. Its own doc block claims the opposite:

> The mode is applied through an open handle (`fchmod`), and the handle is
> confirmed to be a directory first, so a symlink at `parent` cannot have its
> target chmodded — the same discipline as [`harden_existing_ingest_dir`].

That discipline is not actually applied here. `harden_existing_ingest_dir`
(same file, line 284) `lstat`s first via `reject_untrusted_ingest_dir`, refuses
a symlink or non-directory, and only then opens a handle and re-checks
`(dev, ino)` against the `lstat` before `fchmod`. `harden_ingest_parent` does
none of those three steps. A symlink-to-directory planted at the parent path
yields a handle on the *target*, `is_dir()` is true, and `set_permissions`
(fchmod) clears the group/other write bits on the attacker-chosen target — and
the function then returns `Ok(())`, so `create_ingest_dir` proceeds to create
and "harden" the leaf ingest dir *inside* that target and `IngestDir::open`
takes its anchor there. `O_NOFOLLOW` on the leaf does not help: it is the
parent component that was redirected.

**Why it matters**: the whole TASK-2039 defence is "remove the swap capability
from the staging parent"; the `IngestDir` anchor (TASK-2054) is explicitly
documented as complementary to it, not a replacement. A parent that is itself a
symlink defeats the capability half and silently relocates the entire staging
area, while the code reports success. It also mutates permissions on a
directory the caller never named. The mismatch between the documented guarantee
and the code is the more serious half: a reader auditing this path is told the
symlink case is handled.

<!-- scan confidence: verified by reading; compare dir.rs:196-258 against dir.rs:284-313 -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 harden_ingest_parent rejects a symlink (and any non-directory) at the staging parent path before opening it, using the same reject_untrusted_ingest_dir gate harden_existing_ingest_dir uses
- [ ] #2 the fchmod is applied only through a handle whose (dev, ino) matches the lstat taken during rejection, matching harden_existing_ingest_dir
- [ ] #3 a unix test plants a symlink-to-directory at the staging parent and asserts create_ingest_dir fails, the symlink target keeps its original mode, and no leaf ingest dir is created inside the target
- [ ] #4 the doc block's symlink claim either becomes true or is corrected to state the residual
<!-- AC:END -->
