---
id: TASK-2639
title: 'Rust stack: built-in commands for semver, feature-matrix, dependency-floor and doc-id checks'
status: To Do
assignee: []
created_date: '2026-10-26 22:12'
labels:
  - feature
  - rust
  - stack-defaults
dependencies: []
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
- [ ] #1 Each adopted check is documented in docs/commands.md with its required tool
- [ ] #2 A missing tool reports the install hint instead of failing obscurely
<!-- AC:END -->
