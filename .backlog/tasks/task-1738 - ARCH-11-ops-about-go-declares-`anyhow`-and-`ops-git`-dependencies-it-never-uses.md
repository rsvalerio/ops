---
id: TASK-1738
title: >-
  ARCH-11: ops-about-go declares `anyhow` and `ops-git` dependencies it never
  uses
status: Triage
assignee: []
created_date: '2026-08-27 11:13'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - extensions-go/about/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/Cargo.toml:9-17`

**What**: The `[dependencies]` table lists

```toml
ops-git = { workspace = true }
anyhow  = { workspace = true }
```

Neither appears anywhere in `extensions-go/about/src/`:

```
$ grep -rn "anyhow\|ops_git" extensions-go/about/src/
(no matches)
```

The git-remote value that ends up in `ProjectIdentity::repository` (pinned
by `provide_populates_repository_from_git_remote`, lib.rs:278) is resolved
inside `ops_about::identity::provide_identity_from_manifest`, so this crate
never touches `ops-git` itself. Error handling goes through
`DataProviderError`, never `anyhow::Result`.

`linkme` *is* required despite having no textual use — `impl_extension!`
expands to `#[linkme::distributed_slice(...)]` (crates/extension/src/macros.rs:86)
and resolves the path in the calling crate — so it must stay.

Cross-crate context (not a finding against those crates, per review scope):
the same two unused entries appear verbatim in `extensions-node/about`,
`extensions-python/about` and `extensions-java/about`, which is how the
copy propagated. `extensions-rust/about` genuinely uses both.

**Why it matters**: unused dependency edges enlarge the compile graph for
every build of this crate and widen the crate's supply-chain surface for no
benefit. Cargo never warns about them, so they only leave via a deliberate
audit. `ops-git` in particular pulls a first-party crate into the Go
extension's dependency graph, obscuring the actual architecture — a reader
reasonably concludes this crate does its own git resolution.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `anyhow` and `ops-git` are removed from `extensions-go/about/Cargo.toml` and the crate still builds (`cargo build -p ops-about-go --all-targets`)
- [ ] #2 `linkme` is retained, with a short comment recording that `impl_extension!` requires it in the calling crate
- [ ] #3 `cargo clippy --all-targets --workspace -- -D warnings` and `cargo nextest run -p ops-about-go` pass
<!-- AC:END -->
