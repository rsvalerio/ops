---
id: TASK-2639
title: 'Rust stack: built-in commands for semver, feature-matrix, dependency-floor and doc-id checks'
status: Done
assignee: []
created_date: '2026-10-26 22:12'
updated_date: '2026-11-01 00:27'
labels:
  - feature
  - rust
  - stack-defaults
dependencies: []
parent_task_id: 'TASK-2645'
modified_files: []
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
dbsec wraps generic Rust checks in scripts that any Rust repo on ops would want, the way ops msrv is already built in:
- semver: cargo-semver-checks against the latest crates.io release (scripts/semver-check.sh, 94 lines)
- features: cargo-hack, each feature alone plus the default build (scripts/feature-matrix.sh, 82 lines)
- dep-floors: build against the declared minimum versions (scripts/dep-floor-check.sh, 169 lines)
- cog-check: cog check --from-latest-tag, tolerating no tags, plus cog bump --auto --dry-run (scripts/cog-check.sh)
- doc-ids: no TASK-nnnn or review-rule ids in /// or //! comments, across every Rust file including nested workspaces (scripts/doc-id-check.sh, 117 lines). ops's own v0.81-0.82 releases spent many commits removing exactly these tags by hand.

Each should be a stack-gated command with its tool named when missing, as ops deps and ops sec do. Pick per item; they are independent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each adopted check is documented in docs/commands.md with its required tool
- [x] #2 A missing tool reports the install hint instead of failing obscurely

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Adopted: ops semver (cargo-semver-checks), ops feature-matrix (cargo-hack, named so a generic `features` config command stays free) and ops doc-ids (native, no tool). All are stack-gated, composable built-ins documented in docs/commands.md; a missing cargo plugin fails with its install command.

Not adopted:
- dep-floors: needs a nightly toolchain and rewrites Cargo.lock to resolve minimum versions, so it does not fit a single fixed read-mostly command. dbsec's 169-line script carries policy a built-in would have to guess. Repos keep their script.
- cog-check: cocogitto and conventional-commit policy are not Rust-specific, so it is not a Rust stack check, and it would need tag-tolerance logic. A repo can define it in .ops.toml.

Verified: the missing-plugin path ran for real here (cargo-semver-checks is not installed) and is unit-tested. feature-matrix ran on a scratch crate. The passing path of ops semver was not exercised on this machine. doc-ids is not in this repo's gates: it reports 989 ids (TASK-2647).
<!-- SECTION:NOTES:END -->
