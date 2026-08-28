---
id: TASK-1959
title: >-
  SEC-33: every discovered file is read whole into memory with no size cap, so
  one large tracked file OOMs the pre-commit hook
status: To Do
assignee:
  - TASK-2011
created_date: '2026-08-27 15:50'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/text-fixers/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: Medium

**File**: `extensions/text-fixers/src/lib.rs:129` (`run_fixer`), with `extensions/text-fixers/src/trailing.rs:7` and `extensions/text-fixers/src/eof.rs:34`

**What**: `std::fs::read(&path)` loads the entire file into a `Vec<u8>` with no upper bound, for every file discovery returns. `fix_trailing` then allocates a second buffer of `input.len()` (`Vec::with_capacity(input.len())`), so peak resident memory is roughly **2x the largest file in the repo**, held while the file is processed.

Nothing bounds the input:

- there is no `metadata().len()` gate anywhere in this crate (contrast `extensions/config-checkers/src/lib.rs:243`, which at least has an advisory one — the sibling crate that calls `ops_text_fixers::discovery::discover`);
- the binary sniff (binary.rs) rejects only files with a NUL in the first 8 KiB, so a multi-gigabyte NUL-free file (a CSV export, a log, an ndjson dump, a `.sql` seed file, a minified bundle, a Git LFS pointer's real payload if smudged) sails straight through;
- under `--tracked` the path list comes from `git ls-files` and, per the companion SEC-14 finding, is not filtered to regular files, so a symlink to a block device or a FIFO reaches `fs::read` too — an unbounded or never-terminating read.

**Why it matters**: this runs from a git hook, so the failure is an OOM kill or a swap storm during `git commit`, not a clean error. Combined with TASK-1943 (non-atomic write), an allocation failure part-way through a large file's rewrite is one of the ways a file gets left truncated. The input is repository-controlled, which is the SEC-33 trigger: a checked-out repo can dictate the allocation size.

**Suggested fix**: gate on `metadata().len()` before reading — but implement it as part of the open, not as a separate `metadata` call followed by an independent `read` (that shape is precisely the TOCTOU that TASK-1811 flags in config-checkers). Open the file once, `symlink_metadata`/`File::metadata` on the handle, skip and report if it exceeds a documented cap, then read from that same handle. Expose the cap on `FixerOptions` with a sane default so a repo that genuinely wants to fix a 200 MB file can. Skipped-because-too-large files must be reported, not silently dropped (see the ERR-1 finding).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_fixer enforces a documented maximum file size before reading the whole file into memory
- [ ] #2 The size check is performed on an already-open file handle rather than as a metadata call followed by an independent read
- [ ] #3 The cap is configurable through FixerOptions and has a documented default
- [ ] #4 Files skipped for exceeding the cap are reported on the writer and counted in FixerReport, not silently dropped
<!-- AC:END -->
