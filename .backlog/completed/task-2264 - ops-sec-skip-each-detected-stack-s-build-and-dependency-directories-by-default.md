---
id: TASK-2264
title: 'ops sec: skip each detected stack''s build and dependency directories by default'
status: Done
assignee: []
created_date: '2026-09-16 17:01'
updated_date: '2026-09-16 18:48'
labels:
  - feature
  - sec
dependencies: []
parent_task_id: 'TASK-2265'
modified_files:
  - crates/cli/src/sec_cmd.rs
  - crates/core/src/stack/mod.rs
  - README.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`ops sec` runs `trivy fs` / `trivy config` over the whole project root, including build output: `target/` for Rust and Maven, `build/` for Gradle, `node_modules/` and `dist/` for Node/Vite, `.venv/` and `__pycache__/` for Python, `.terraform/` for Terraform, and so on. That is slow (about 16s on dbsec, mostly `target/`), scans generated artefacts rather than source, and races with concurrent builds: Trivy aborts when a file vanishes mid-walk.

`sec_cmd.rs` already has a skip list for its *detection* walk (the constant near line 45, which mirrors text-fixers' `SKIP_DIRS`), but that list is never passed to Trivy.

Proposal: each stack declares its default build and dependency directories, and `ops sec` passes those for every detected stack, plus the VCS dirs, as `--skip-dirs` to each Trivy invocation. Nested workspaces also need covering, e.g. `fuzz/target` in dbsec, where `fuzz/` is its own Cargo workspace: skip matching directory names at any depth, or discover per-manifest target dirs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every stack declares its default build/dependency directories (Rust: target; Java Maven: target; Java Gradle: build, .gradle; Node/Vite: node_modules, dist; Python: .venv, venv, __pycache__, build, dist; Go: vendor; Terraform: .terraform; Ansible: collections and roles caches if applicable)
- [x] #2 `ops sec` passes the skip dirs for every detected stack, plus .git, to every Trivy scan (secret, vuln, config)
- [x] #3 The directories are skipped at any depth, so a nested workspace's target/ is covered
- [x] #4 There is a way to opt out or extend the list (e.g. a `[sec]` config section or `--no-default-skips`)
- [x] #5 `ops --dry-run sec` / the plan output shows the skipped directories
- [x] #6 Detection and Trivy use the same skip list, so the two cannot drift apart
- [x] #7 README documents the default skip list per stack

<!-- AC:END -->
