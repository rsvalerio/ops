---
id: TASK-1785
title: >-
  TEST-6: strict workspace-root symlink rejection path has no test; the test
  named for it builds no symlink
status: To Do
assignee:
  - TASK-1994
created_date: '2026-08-27 11:23'
updated_date: '2026-08-28 14:12'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/tests/find_root.rs
  - extensions-rust/cargo-toml/src/workspace_root.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/tests/find_root.rs:233-263` (`find_root_strict_skips_off_chain_canonical_ancestor`), covering `extensions-rust/cargo-toml/src/workspace_root.rs:186-212`.

**What**: `find_workspace_root_strict_with_depth` exists for exactly one reason — the SEC-25 / TASK-1204 defence that rejects a candidate `Cargo.toml` whose canonical parent is off the canonical start's ancestor chain. That rejection is the `else` arm at `workspace_root.rs:193-202` (emit `tracing::warn!`, return `CandidateAction::Skip`).

No test reaches that arm. The test whose doc comment describes the scenario —

> "a sibling symlink at an intermediate ancestor would redirect a lexical walk into an attacker tree. The strict variant inspects each candidate's *canonical* parent and skips the redirected ancestor"

— creates **no symlink at all**. Its body builds a plain `real_root/legit/leaf/Cargo.toml`, then asserts that both the strict and the lenient variant return `leaf`. It verifies only the *accept* path, and asserts nothing the lenient walk does not already satisfy.

The only symlink-planting test in the file, `find_root_lenient_follows_symlinked_ancestor_into_attacker_tree` (`:209-231`), deliberately pins the **lenient** variant's vulnerable behaviour and never calls the strict variant.

Lineage: TASK-1503 (Done, LOW) shrank a 110-line test then named `find_root_strict_rejects_symlinked_ancestor_planting`. The rename to `find_root_strict_skips_off_chain_canonical_ancestor` survived; the symlink scenario did not.

**Why it matters**: TEST-6 / classification "Useless — creates false confidence in coverage metrics". The test name, doc comment, and the SEC-25 annotations all read as coverage of a security control that is in fact unexecuted. `find_workspace_root_strict` is the entry point used by `extensions-rust/about/src/query.rs:433` and `extensions-rust/create-review-tasks/src/provider.rs:22` — i.e. the callers that opted *into* the hardened variant are relying on an untested branch. Deleting the entire `else` arm today would leave the suite green.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test constructs a layout where an intermediate ancestor is a symlink into an off-chain (attacker) tree containing a [workspace] Cargo.toml, and asserts find_workspace_root_strict does NOT return that manifest
- [ ] #2 The paired assertion that find_workspace_root (lenient) DOES follow the same symlink is retained, so the two variants are contrasted in one place
- [ ] #3 A test covers the CandidateAction::Skip arm reached via a canonicalize failure on a candidate ancestor (workspace_root.rs:204-211)
- [ ] #4 find_root_strict_skips_off_chain_canonical_ancestor is either renamed to match what it actually asserts, or extended to assert the rejection its doc comment describes
<!-- AC:END -->
