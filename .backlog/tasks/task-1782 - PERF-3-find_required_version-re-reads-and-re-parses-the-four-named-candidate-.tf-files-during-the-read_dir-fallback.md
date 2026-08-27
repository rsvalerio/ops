---
id: TASK-1782
title: >-
  PERF-3: find_required_version re-reads and re-parses the four named candidate
  .tf files during the read_dir fallback
status: Triage
assignee: []
created_date: '2026-08-27 11:23'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:89-148` (`find_required_version`)

**What**: The function first probes four named candidates — `versions.tf`, `main.tf`, `terraform.tf`, `version.tf` (`:90-101`). When none of them carries a `required_version`, the fallback enumerates *every* `*.tf` in the workspace root (`:124-148`), and that listing necessarily includes the same four files. Each is opened, read into a `String`, run through `strip_block_comments` (which allocates a second full copy of the content) and scanned again.

The common case is the one that pays: a terraform project with a `main.tf` and no `required_version` anywhere reads and parses `main.tf` twice on every `ops about` invocation. The fallback also has no bound on how many `.tf` files it will open (each read is capped at `MAX_MANIFEST_BYTES`, but the file count is not), so a root directory with many generated `.tf` files multiplies the work.

**Why it matters**: Small in absolute terms, but it is pure waste on the hot path of a command whose whole job is to be instant, and it is trivially avoidable — skip paths whose file name is already in the candidate list (or drive the whole thing off one sorted listing, with the named candidates ordered first). The duplicated `strip_block_comments` allocation compounds it: every re-read is two allocations of the file's length.

**Secondary**: `strip_block_comments` (`:309-360`) allocates and copies the entire file even when the content contains no `/*` at all. An early `if !content.contains("/*") { return Cow::Borrowed(content) }` (returning `Cow<str>`) would make the common case allocation-free.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The read_dir fallback does not re-read a .tf file already probed by the named-candidate loop
- [ ] #2 strip_block_comments avoids allocating a full copy when the content contains no block comment
- [ ] #3 Behaviour is unchanged: the alphabetically-first .tf carrying a constraint still wins, and the existing determinism/case-insensitivity tests pass
<!-- AC:END -->
