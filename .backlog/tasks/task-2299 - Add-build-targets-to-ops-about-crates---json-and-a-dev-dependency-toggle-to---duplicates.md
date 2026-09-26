---
id: TASK-2299
title: 'Add build targets to ops about crates --json and a dev-dependency toggle to --duplicates'
status: Triage
assignee: []
created_date: '2026-09-26 19:08'
labels:
  - code-review-rust
  - feature
dependencies: []
modified_files:
  - extensions/about/src/units.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/deps_provider.rs
  - crates/cli/src/args.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs`, `extensions-rust/about/src/deps_provider.rs`

**What**: TASK-2282's description asked for each crate's targets in `ops about crates --json`; the shipped document carries name, version, repo-relative manifest dir and in-tree flag (the ACs) but no targets — the manifest-backed units provider has no view of auto-discovered targets (needs `cargo metadata` `targets`). TASK-2288 says dev-only duplicates are excluded "by default"; the shipped `--duplicates` always excludes them, with no flag to include them.

**Why it matters**: skills that size crates by target (bin/lib/test) still fall back to `cargo metadata` + jq; a caller auditing dev-dependency duplicates has no option.

**Origin**: discovered during TASK-2292 while fixing TASK-2282 and TASK-2288.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops about crates --json lists each crate's targets (kind + name) without checkout-dependent paths
- [ ] #2 ops about dependencies --duplicates accepts a flag that includes dev-only duplicates
<!-- AC:END -->
