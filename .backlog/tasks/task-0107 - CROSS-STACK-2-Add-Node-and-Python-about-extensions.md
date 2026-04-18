---
id: TASK-0107
title: 'CROSS-STACK-2: Add Node and Python about extensions'
status: Triage
assignee: []
created_date: '2026-04-18 10:13'
labels:
  - cross-stack
  - feature
  - about
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: Bring Node and Python into the `about` extension family by adding `extensions-node/about` and `extensions-python/about` crates that register `project_identity` and `project_units` data providers.

**Why it matters**: `ops about` works on Rust, Go, and Java (Maven/Gradle) today but silently degrades on Node and Python despite both being first-class `Stack` variants with default commands and detection. Closing this gap gives every supported stack a usable `ops about` card out of the box.

**Scope**:
- `extensions-node/about`: parse `package.json` for name/version/description/author/license/repository/homepage; register `project_identity` provider. Workspaces (npm/yarn/pnpm) map to `project_units`.
- `extensions-python/about`: parse `pyproject.toml` (PEP 621) with fallback to `setup.py`/`setup.cfg`; register `project_identity` provider. Monorepo/namespace packages map to `project_units` if applicable.
- Follow patterns in `extensions-rust/about/src/{identity,units}.rs` and `extensions-go/about/src/{lib,modules}.rs`.
- Wire into CLI feature gates mirroring `stack-rust`/`stack-go`.

**Context**: Parity review identified Node and Python as the biggest hole. See `/Users/rsvaleri/.claude/plans/yes-lets-plam-and-wild-tiger.md`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `ops about` on a Node project renders a ProjectIdentity card from package.json
- [ ] #2 `ops about` on a Python project renders a ProjectIdentity card from pyproject.toml
- [ ] #3 `ops about modules` shows workspace members on Node projects with workspaces
- [ ] #4 New extensions register `project_identity` and (where applicable) `project_units` providers
- [ ] #5 Tests cover identity parsing for both stacks; extension registration verified
<!-- AC:END -->
