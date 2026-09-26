---
id: TASK-2282
title: 'Add --json to ops about crates, loc and dependencies, and to ops backlog search'
status: Done
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 19:08'
labels:
  - feature
  - cli
  - about
  - backlog
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2292'
modified_files:
  - crates/cli/src/about_cmd.rs
  - extensions/about/src/units.rs
  - extensions/about/src/loc.rs
  - extensions/about/src/deps.rs
  - crates/backlog/src/cmd/search.rs
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: machine-readable output (with a `schemaVersion`, like `backlog task list/view --json`) for:

- `ops about crates`: for each member, name, version, the repo-relative manifest dir, whether it is in-tree, and its targets
- `ops about loc`: production, test and example line counts per crate
- `ops about dependencies`: the dependency tree
- `ops backlog search`: the matching tasks with id, status, labels and modifiedFiles. Today it has only `--plain`

**Why**: skills re-derive what ops already knows. `rust-make-clippy-pedantic` has a whole reference page (`extraction.md`) on normalizing `cargo metadata` output with `jq`. It covers absolute `path+file://` package ids that change per checkout or worktree, and members outside the checkout leaking absolute paths into `--modified-file`. `code-review-rust`/`-web` size crates with `wc`/`ls`/`tree`. The dedupe step in every finding-filing skill parses `ops backlog search --plain`.

**Used by**: code-review-rust, rust-make-clippy-pedantic, rust-make-build-fast, code-review-triage.
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `ops about crates --json` lists members with name, version, repo-relative manifest dir and an in-tree flag; paths never depend on the checkout location
- [x] #2 `ops about loc --json`, `ops about dependencies --json` and `ops backlog search --json` exist, each with a schemaVersion
- [x] #3 Tests pin the JSON shape of each

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shipped: about crates/loc/dependencies --json (schemaVersion 1, kinds about-crates/about-loc/about-dependencies) and backlog search --json (kind search). crates: name (package name, falls back to display), version (workspace-inherited versions now resolved), repo-relative manifestDir, inTree. loc: workspace regions + per-crate split by longest member prefix. Targets (description only, not an AC) filed as TASK-2299.
<!-- SECTION:NOTES:END -->
