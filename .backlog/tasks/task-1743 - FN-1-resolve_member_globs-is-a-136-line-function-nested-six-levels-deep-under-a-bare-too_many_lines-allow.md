---
id: TASK-1743
title: >-
  FN-1: resolve_member_globs is a 136-line function nested six levels deep under
  a bare too_many_lines allow
status: To Do
assignee:
  - TASK-2003
created_date: '2026-08-27 11:13'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - complexity
dependencies: []
modified_files:
  - extensions/about/src/workspace.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:32-169`

**What**: `resolve_member_globs` spans lines 33-169 (136 lines, ~100 excluding comments) and carries `#[allow(clippy::too_many_lines)]` at line 32 with **no reason attached**. Nesting reaches six levels on the hot path:

```
for member in members                     (1)
  if let Some((prefix, suffix)) = ...     (2)
    match std::fs::read_dir(&parent)      (3)
      Ok(entries) => for entry in entries (4)
        let entry = match entry {         (5)
          if let Some(manifest) = ...     (6)
            let rel_string = path.strip_prefix(root).map_or_else(...)
```

FN-2's limit is 4. The body mixes five distinct concerns that each have a natural name: member-value validation (`..` rejection, lines 50-61), glob-shape validation (lines 62-84), directory enumeration with per-entry error classification (lines 90-158), relative-path derivation (lines 115-137), and post-processing (exclude filtering, sort, dedup, lines 163-168). Only the last of those — `recover_relative_path` — has already been extracted, and its doc comment states the motive plainly: "Split out of the loop body so the caller's `map_or_else` keeps the common `strip_prefix` success on one line instead of trailing 25 lines of recovery." The same argument applies to the four blocks still inline.

The lint suppression is the aggravating detail. Per READ-10, `#[allow]` with a specific, accurate reason documents a deliberate policy exception and earns a severity reduction; a bare `#[allow]` with no rationale is a silenced warning, and it has already outlived at least five separate changes to this function (TASK-0517, TASK-0942, TASK-1069, TASK-1070, TASK-1071, TASK-1149 are all cited in its comments). Each of those landed by adding another block inside the same body rather than by extracting one, which is the growth pattern the lint was there to interrupt.

**Why it matters**: this is the crate's most security-relevant function — it validates operator-supplied glob patterns against path traversal and drives filesystem enumeration — and it is the one function a reviewer cannot hold in their head. The `..` guard, the glob-shape guard, and the exclude fail-closed policy are three independent security decisions currently interleaved with IO error plumbing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 resolve_member_globs is under 50 lines and reads as orchestration: validate member, expand glob, filter excludes, sort/dedup
- [ ] #2 Member-value validation and glob-shape validation are each a named predicate or helper that can be unit-tested without touching the filesystem
- [ ] #3 Nesting on every path is at most 4 levels
- [ ] #4 The #[allow(clippy::too_many_lines)] is removed; if any suppression remains it is #[expect] with a written reason per docs/clippy.md
<!-- AC:END -->
