---
id: TASK-2013
title: 'ARCH-11: ops-about-node declares anyhow and ops-git dependencies it never uses'
status: To Do
assignee:
  - TASK-2049
created_date: '2026-08-28 15:32'
updated_date: '2026-08-29 11:35'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - extensions-node/about/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/Cargo.toml:12,16`

**What**: `[dependencies]` lists `ops-git = { workspace = true }` and
`anyhow = { workspace = true }`. Neither is referenced anywhere in
`extensions-node/about/src/` (`grep -rn "anyhow\|ops_git"` matches only the
unrelated test name `normalize_drops_git_plus_non_http_schemes`). The git
remote that reaches `ProjectIdentity::repository` is resolved inside
`ops_about::identity::provide_identity_from_manifest`, and error handling
goes through `DataProviderError`, never `anyhow::Result`.

`linkme` *is* required despite having no textual use — `impl_extension!`
expands to `#[linkme::distributed_slice(...)]` and resolves the path in the
calling crate — so it must stay.

**Why it matters**: unused dependency edges enlarge the compile graph and widen
the supply-chain surface for no benefit; Cargo never warns about them. `ops-git`
in particular pulls a first-party crate into the Node extension's graph, so a
reader reasonably concludes this crate does its own git resolution.

The identical pair was already filed for the java (TASK-1749), python
(TASK-1760), terraform (TASK-1786) and go (TASK-1738, fixed) siblings; the
node copy is the remaining one.

**Origin**: discovered during TASK-1989 while fixing TASK-1738.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 anyhow and ops-git are removed from extensions-node/about/Cargo.toml and the crate still builds (cargo build -p ops-about-node --all-targets)
- [ ] #2 linkme is retained, with a short comment recording that impl_extension! requires it in the calling crate
- [ ] #3 cargo clippy --all-targets --workspace -- -D warnings and cargo nextest run -p ops-about-node pass
<!-- AC:END -->
