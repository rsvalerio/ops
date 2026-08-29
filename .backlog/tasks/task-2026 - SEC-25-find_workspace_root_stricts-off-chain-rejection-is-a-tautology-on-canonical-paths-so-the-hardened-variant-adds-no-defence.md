---
id: TASK-2026
title: >-
  SEC-25: find_workspace_root_strict's off-chain rejection is a tautology on
  canonical paths, so the hardened variant adds no defence
status: To Do
assignee:
  - TASK-2041
created_date: '2026-08-28 20:16'
updated_date: '2026-08-29 11:35'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/workspace_root.rs
  - extensions-rust/cargo-toml/src/tests/find_root.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/workspace_root.rs` (`strict_candidate_action`, reached from `find_workspace_root_strict_with_depth`)

**What**: the strict variant's whole reason to exist is the SEC-25 / TASK-1204 check `start_canonical.starts_with(&canonical_parent)`. But `walk_ancestors` canonicalizes `start` once up front and then reaches every candidate via `Path::parent` on that already-canonical path. Every lexical ancestor of a canonical path is itself canonical, so `canonicalize(current) == current` and the `starts_with` test is unconditionally true on any quiescent filesystem. The `else` arm (and the `Err` arm) can only fire if an ancestor is swapped for a symlink *during* the walk — a TOCTOU race that cannot be constructed deterministically from a test.

Concretely, for the layout the doc comment describes — an ancestor symlink redirecting into an attacker tree — `find_workspace_root_strict` returns the attacker's `[workspace]` manifest, exactly like the lenient variant. TASK-1785 pins that behaviour in `find_root_strict_also_follows_symlink_inside_the_start_path` and covers the two `Skip` arms through an injected canonicalizer, because the filesystem cannot reach them.

**Why it matters**: `extensions-rust/about/src/query.rs` and `extensions-rust/create-review-tasks/src/provider.rs` opted into the hardened entry point and are relying on a control that is inert. Two candidate fixes, both needing a deliberate decision rather than a drive-by change in a test-coverage task:

1. Re-anchor the check to the caller's *pre-canonical* `start` path (reject a root that is not a lexical ancestor of what the caller asked about). This is what "outside the user's intended logical path" actually means — but it would also reject every legitimate cwd reached through a symlink, so it needs a survey of real callers first.
2. Canonicalize the candidate `Cargo.toml` *file* and reject one whose canonical parent is not the candidate directory. This catches a planted `Cargo.toml` **symlink** into an attacker tree, is deterministically testable, and does not affect ordinary manifests — but it is a new security control, not the one currently documented.

Either way, the doc comment on `find_workspace_root_strict` must stop promising a defence the code does not provide. TASK-1785 added a "Scope of the guarantee" section as an interim measure.

**Origin**: discovered during TASK-1994 while fixing TASK-1785.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 find_workspace_root_strict either enforces a check that a filesystem-level test can drive to rejection, or its doc comment and the SEC-25 annotations stop claiming a symlink-planting defence
- [ ] #2 A test constructs a real on-disk layout in which find_workspace_root_strict rejects a manifest that find_workspace_root accepts
- [ ] #3 The decision (re-anchor to the pre-canonical start, validate the manifest file's own canonical path, or retire the strict variant) is recorded on the function
<!-- AC:END -->
