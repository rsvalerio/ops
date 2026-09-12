---
id: TASK-2261
title: 'READ-13: doc/comment change-journal narration survives in files that fell outside wave14''s per-task file lists'
status: Triage
assignee: []
created_date: '2026-09-10 19:32'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/tests.rs
  - crates/runner/src/command/events.rs
  - crates/runner/src/command/resolve.rs
  - crates/runner/src/command/parallel.rs
  - crates/runner/src/command/secret_patterns.rs
  - crates/extension/src/context.rs
  - extensions/about/src/units.rs
  - extensions/about/src/text_util.rs
  - extensions/duckdb/src/sql/ingest/orchestrator.rs
  - extensions-java/about/Cargo.toml
  - extensions-terraform/about/Cargo.toml
  - extensions-go/about/Cargo.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/tests.rs`, `crates/runner/src/command/events.rs`, `crates/runner/src/command/resolve.rs`, `crates/runner/src/command/parallel.rs`, `crates/runner/src/command/secret_patterns.rs`, `crates/extension/src/context.rs`, `extensions/about/src/units.rs`, `extensions/about/src/text_util.rs`, `extensions/duckdb/src/sql/ingest/orchestrator.rs`, `extensions-java/about/Cargo.toml`, `extensions-terraform/about/Cargo.toml`, `extensions-go/about/Cargo.toml`

**What**: wave14 (TASK-2248) cleared the READ-13 change-journal pattern from every file named in its member tasks' `Modified files` lists. The same pattern survives in sibling files those lists did not name, because each finding was scoped to the file it was scanned in rather than to the crate.

Current counts of `TASK-nnnn` citations, plus the usual `previously` / `used to` / `the predecessor` / `pre-fix` framing and rule-ID doc prefixes (`PERF-x`, `CONC-x`, `SEC-x`, `TEST-x`, `ARCH-x`, ...):

- `extensions-rust/cargo-update/src/tests.rs` — 48 (excluded from TASK-2155, which listed only `lib.rs`)
- `crates/runner/src/command/parallel.rs` — 45; `resolve.rs` — 19; `events.rs` — 8; `secret_patterns.rs` — 9 (TASK-2101 listed only build/results/mod/render_config/progress_state)
- `crates/extension/src/context.rs` — 28 (TASK-2099 listed only lib/data/error)
- `extensions/about/src/text_util.rs` — 39; `units.rs` — 15 (TASK-2070 listed only the four warn-site files)
- `extensions/duckdb/src/sql/ingest/orchestrator.rs` — 21 (TASK-2125 listed only `sql/mod.rs` and `ingest/sidecar.rs`)
- three `about` crate `Cargo.toml` files carry `ARCH-11 / TASK-nnnn` comments (1, 2 and 1 respectively)

**Why it matters**: the workspace is now inconsistent — a reader of `ops-runner` gets end-state docs in `build.rs` and a change journal in `parallel.rs`. `crates/runner/src/command/parallel.rs` and `extensions/about/src/text_util.rs` are the two densest remaining sites. `--document-private-items` renders most of this, and the cited task ids point at a backlog the reader cannot open.

**Origin**: discovered during TASK-2248 while fixing TASK-2155, TASK-2099, TASK-2101, TASK-2070 and TASK-2125; each was closed against its own file list, so these siblings had no owner.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No TASK-nnnn reference or review rule-ID prefix remains in a doc comment or inline comment in the listed files
- [ ] #2 Change-history framing (previously / used to / the predecessor / pre-fix / the previous X) is replaced by statements of current behaviour, with the load-bearing rationale kept
- [ ] #3 cargo doc --workspace --document-private-items still builds with no new warnings and no broken intra-doc links
<!-- AC:END -->
