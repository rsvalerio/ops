---
id: TASK-2283
title: 'Add ops clippy --findings-json: one normalized row per Clippy diagnostic for surveys'
status: To Do
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 18:27'
labels:
  - feature
  - cli
  - clippy
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2293'
modified_files:
  - crates/cli/src/args.rs
  - crates/core/src/.default.rust.ops.toml
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: a survey mode for Clippy that is not a gate: it runs `cargo clippy --message-format=json` (never `-D warnings`) with caller-supplied lint flags. It emits one row per diagnostic:

- lint name, with the `clippy::` prefix stripped
- the package as `name@version` plus its repo-relative manifest dir, never the raw `package_id`
- target name and kind
- primary-span file (repo-relative), line and column; a diagnostic with no span is attributed to the crate's `Cargo.toml` at line 0
- the message, verbatim

Spans outside the repository (registry, `OUT_DIR`, generated files) are dropped and counted, and plain rustc warnings are counted separately.

**Why**: this is exactly the `jq` pipeline in `rust-make-clippy-pedantic`'s `extraction.md`. Each of its rules exists because the obvious alternative silently corrupts the finding set: spanless diagnostics shifting columns, `file:line` keys merging distinct findings, absolute package ids re-filing everything from another checkout. Owning it in ops makes it tested code instead of prose an agent must reproduce.

**Used by**: rust-make-clippy-pedantic.
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rows carry lint, name@version, repo-relative manifest dir, target, file, line, column and verbatim message
- [ ] #2 Spanless diagnostics are attributed to the crate manifest; out-of-tree spans are dropped and counted
- [ ] #3 Output is identical from two checkouts of the same commit at different paths
- [ ] #4 The existing `ops clippy` gate is unchanged
<!-- AC:END -->
