---
id: TASK-1342
title: >-
  API-1: pub type Subcommand = CoreSubcommand is a dead alias whose doc comment
  implies a no-longer-existing prefix flattening
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-12 16:41'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:205-207`

**What**: `pub type Subcommand = CoreSubcommand;` is a public type alias with a single underlying enum. The doc comment claims it "flattens CoreSubcommand so `ops verify` works directly" and that "the `ops` prefix from `cargo ops ...` is stripped before parsing" — but the alias adds nothing (no second variant, no transformation); the strip logic lives in `preprocess_args` at line 239.

**Why it matters**: Misleading API surface — readers expect the alias to hide structural variance (e.g., two enums collapsed) and instead find a trivial rename. Either inline `CoreSubcommand` at the single use site (`Cli::subcommand: Option<Subcommand>` at line 49) or remove the alias and the stale doc comment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Alias removed and Cli::subcommand uses CoreSubcommand directly (or doc comment rewritten to reflect actual purpose)
- [ ] #2 cargo build and cargo test --workspace pass
<!-- AC:END -->
