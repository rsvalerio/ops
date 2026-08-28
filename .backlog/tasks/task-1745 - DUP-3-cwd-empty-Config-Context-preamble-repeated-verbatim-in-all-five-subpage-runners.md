---
id: TASK-1745
title: >-
  DUP-3: cwd + empty-Config + Context preamble repeated verbatim in all five
  subpage runners
status: To Do
assignee:
  - TASK-2003
created_date: '2026-08-27 11:13'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/about/src/providers.rs
  - extensions/about/src/units.rs
  - extensions/about/src/coverage.rs
  - extensions/about/src/deps.rs
  - extensions/about/src/code.rs
  - extensions/about/src/loc.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:56-58`, `extensions/about/src/coverage.rs:73-75`, `extensions/about/src/deps.rs:37-39`, `extensions/about/src/code.rs:80-82`, `extensions/about/src/loc.rs:200-202`

**What**: Five `run_about_*_with` functions open with the same three lines, byte for byte:

```rust
let cwd = std::env::current_dir()?;
let config = std::sync::Arc::new(ops_core::config::Config::empty());
let mut ctx = Context::new(config, cwd);
```

`lib.rs:118-120` is a sixth near-copy (it uses the caller-supplied `cwd` and then sets `ctx.refresh`).

`providers.rs` exists precisely to collapse this class of repetition — its module doc says "every subpage repeats the same ... warm-up loop and the same triadic ... sequence. Centralising both here keeps the four subpages aligned and makes drift between their warm-up lists visible at the call site" (DUP-1 / TASK-0464). It centralised `warm_providers` and `load_or_default` and stopped one step short of the context construction that precedes them, so each runner still open-codes it.

Five occurrences of an identical pattern crosses DUP-3's 3+ threshold. Two concrete drift surfaces it leaves open:

1. Every runner silently decides that the about subpages run against `Config::empty()` rather than the loaded project config. That decision is made in five places with no comment in any of them; changing it (or making it conditional) means finding all five.
2. `std::env::current_dir()?` is propagated bare from five sites — an ERR-4 gap replicated five times over (`lib.rs`'s `run_about` avoids it entirely by taking `cwd` as a parameter, which is the better shape).

A `fn subpage_context() -> anyhow::Result<Context>` in `providers.rs` collapses all five and gives the `Config::empty()` decision and the `current_dir` context a single home.

**Why it matters**: low severity on its own, but it is the residue of a dedup task that was left half-finished, and it is the reason the `current_dir` error message is unattributable in five different subcommands rather than one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single helper in providers.rs builds the subpage Context; all five run_about_*_with functions call it instead of open-coding the three lines
- [ ] #2 The helper attaches context to the current_dir failure (ERR-4) so the error names the subcommand or at least the operation
- [ ] #3 The choice of Config::empty() over the loaded project config is documented once, at the helper
<!-- AC:END -->
