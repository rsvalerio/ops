---
id: TASK-1839
title: >-
  PATTERN-1: display-name fallback collapses distinct workspace members to the
  same review-target name, and the name is the created task's only identity
status: Done
assignee:
  - TASK-1996
created_date: '2026-08-27 15:22'
updated_date: '2026-08-28 15:13'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:27-40`

**What**: When a member's manifest cannot be read or parsed, `provide` names the review target with `format_unit_name(member)` (provider.rs:31-37). That helper returns the capitalized **last path segment only** — `extensions/about/src/cards.rs:49-61` strips a `**/` prefix, takes `split('/').next_back()`, and upper-cases the first char. So `crates/parser` and `tools/parser` both become `Parser`.

Nothing downstream restores the distinction:

- `resolved_workspace_members` dedups member **paths**, not names (`extensions-rust/about/src/query.rs`, the `resolved.dedup()` tail carrying the PATTERN-1 / TASK-1042 comment). Two different paths are two different members and both survive.
- The consumer's contract documents the invariant this provider is supposed to uphold: `ReviewTarget.name` is "a display name (**unique per workspace**, e.g. the cargo package name)" — `extensions/create-review-tasks/src/lib.rs:56-64`. Nothing on either side enforces it.
- The engine renders the subtask title as `REVIEW: Run skill {skill} against {name}` (`extensions/create-review-tasks/src/lib.rs:199-202`) and `PlannedSubtask.path` is explicitly "report context only, never part of a filename" (lib.rs:175-176). The written task file carries only id, title, and parent link — `render_task_file(&mut handle, file.id, file.title, stamp, file.subtask_of)` at lib.rs:~330. **The name is the only identity the created backlog task has**; the member path never reaches it.

Result: two backlog subtasks with byte-identical titles (`REVIEW: Run skill code-review-rust against Parser`) and no way for the agent or operator who picks one up to tell which crate it means — while a third crate silently never gets reviewed under a distinguishable name.

Reachability is not limited to hand-written literal members. Glob-expanded members must have a `Cargo.toml` that *exists* (`expand_member_glob` skips dirs without one), but a manifest that exists and is unreadable (permissions, SEC-33 byte cap) or unparseable still takes the fallback — the crate's own `member_with_malformed_manifest_falls_back_to_display_name` test (provider.rs:155-175) exercises exactly that path. Two same-named leaf dirs under two glob prefixes, both with malformed manifests, collide.

**Why it matters**: the whole point of this provider is to hand the engine one distinguishable review unit per crate, and the fallback silently merges identities instead of preserving them. The information needed to disambiguate is already in hand at the call site — `member` is the unique key. Making the fallback name derive from the full member path (e.g. `crates/parser`), or appending the path when a name repeats, keeps every target addressable without touching the happy path where cargo package names are already unique.

<!-- scan confidence: verified against format_unit_name (cards.rs:49-61), resolved_workspace_members dedup, and render_task_file's field list -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The display-name fallback yields a distinct name for two members whose last path segment is equal (e.g. crates/parser and tools/parser)
- [x] #2 A test builds a workspace with two same-leaf-named members whose manifests are unparseable and asserts the two emitted target names differ
- [x] #3 The uniqueness expectation stated on ops_create_review_tasks::ReviewTarget::name is referenced in a comment or doc at the point the fallback name is produced
<!-- AC:END -->
