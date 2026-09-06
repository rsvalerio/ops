---
id: TASK-1512
title: >-
  PERF-3: manifest_declares_workspace parses the whole Cargo.toml just to check
  for a top-level [workspace] key
status: Done
assignee:
  - TASK-1643
created_date: '2026-05-18 19:57'
updated_date: '2026-05-25 16:49'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:571-610`

**What**: `manifest_declares_workspace` is called from inside the per-ancestor loop of both `find_workspace_root_with_depth` and `find_workspace_root_strict_with_depth` (up to MAX_ANCESTOR_DEPTH = 64 candidates per resolve). For every candidate it:

1. `read_capped_to_string(path)` — reads the entire file (capped, but still up to the manifest cap).
2. `toml::from_str::<toml::Value>(&content)` — fully parses the entire TOML AST (all tables, dependency maps, profiles, metadata).
3. Then asks one question: `value.as_table().is_some_and(|t| t.contains_key("workspace"))`.

A full parse of a typical workspace root Cargo.toml allocates dozens of `BTreeMap` / `Vec` / `String` nodes that are immediately dropped. On a deep walk (member crate `cd crates/foo/src/bar`, walk up 4-6 ancestors, each with a real Cargo.toml), this is 4-6 full TOML parses per `provide()` call, executed inside the data-provider hot path before the typed cache ever kicks in.

**Why it matters**: PERF-3 — avoidable allocations in a frequently-called helper. The cheaper alternative is either:

- a line-level scan for `^\[workspace\]` / `^\[workspace.`,
- or a streaming TOML lex that bails on the first top-level header named `workspace`.

For correctness this still has to tolerate `[workspace]` inside a string or comment, but a regex like `(?m)^\[workspace(\]|\.|\s)` is sufficient and avoids the full parse. The existing `tracing::warn!` on parse failure still fires if the file fails to TOML-validate when it is selected as the actual root (the typed parse in `provide_typed` will surface the error).

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 manifest_declares_workspace does not perform a full toml::Value parse for the decision
- [ ] #2 behaviour preserved: a file containing only [workspace.metadata] still counts as declaring a workspace (matches Cargo)
- [ ] #3 a file with a string literal containing the substring [workspace] does NOT register as declaring a workspace
- [ ] #4 new test in src/tests/find_root.rs pins both edge cases
<!-- AC:END -->
