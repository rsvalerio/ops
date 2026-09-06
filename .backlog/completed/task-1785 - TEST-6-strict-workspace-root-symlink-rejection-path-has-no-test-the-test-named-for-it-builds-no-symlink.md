---
id: TASK-1785
title: >-
  TEST-6: strict workspace-root symlink rejection path has no test; the test
  named for it builds no symlink
status: Done
assignee:
  - TASK-1994
created_date: '2026-08-27 11:23'
updated_date: '2026-08-28 20:18'
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
- [x] #1 A test constructs a layout where an intermediate ancestor is a symlink into an off-chain (attacker) tree containing a [workspace] Cargo.toml, and asserts find_workspace_root_strict does NOT return that manifest
- [x] #2 The paired assertion that find_workspace_root (lenient) DOES follow the same symlink is retained, so the two variants are contrasted in one place
- [x] #3 A test covers the CandidateAction::Skip arm reached via a canonicalize failure on a candidate ancestor (workspace_root.rs:204-211)
- [x] #4 find_root_strict_skips_off_chain_canonical_ancestor is either renamed to match what it actually asserts, or extended to assert the rejection its doc comment describes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-1994.

AC #1 satisfied by substitution. The literal criterion ("strict does NOT return the
attacker manifest") is not satisfiable against the current implementation:
walk_ancestors canonicalizes `start` once up front, so every lexical ancestor it then
visits is already canonical and `start_canonical.starts_with(canonicalize(current))` is
unconditionally true. The off-chain rejection arm — the entire reason the strict variant
exists — is therefore unreachable from any deterministic on-disk layout, and so is the
canonicalize-failure arm (walk_ancestors would have failed on `start` first). Only a
TOCTOU swap during the walk can reach either.

What was done instead:
- Extracted the per-candidate decision into `workspace_root::strict_candidate_action`
  with the canonicalizer injected, and covered BOTH Skip arms deterministically
  (`strict_candidate_action_skips_off_chain_canonical_parent`,
  `strict_candidate_action_skips_ancestor_that_fails_to_canonicalize`) plus both accept
  arms so the Skip tests cannot pass vacuously. Deleting either arm now fails the suite.
- AC #2: `find_root_strict_also_follows_symlink_inside_the_start_path` builds the
  symlinked-attacker-tree layout and pins strict and lenient side by side, so the
  contrast lives in one place. It pins that they AGREE, which is the true behaviour.
- AC #4: renamed to `find_root_strict_accepts_on_chain_canonical_ancestor`, matching
  what the body actually asserts.
- Added a "Scope of the guarantee" section to `find_workspace_root_strict` so the doc
  no longer promises a defence the code does not provide.

The underlying defect — the check is inert — is out of scope for a test-coverage task
and is filed as TASK-2026 (Triage, High).
<!-- SECTION:NOTES:END -->
