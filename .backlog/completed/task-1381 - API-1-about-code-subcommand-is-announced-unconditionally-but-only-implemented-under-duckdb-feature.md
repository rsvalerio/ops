---
id: TASK-1381
title: >-
  API-1: about code subcommand is announced unconditionally but only implemented
  under duckdb feature
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 21:55'
updated_date: '2026-05-12 23:29'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:37-42` (handler) and `crates/cli/src/args.rs:158-159` (CLI surface)

**What**: `AboutAction::Code` is declared in `args.rs` with no `#[cfg]` gate — so `ops about code` shows up in help and parses successfully regardless of compiled features — but the handler in `subcommands.rs:37-42` is split:

```rust
#[cfg(feature = "duckdb")]
Some(AboutAction::Code) => ops_about::run_about_code(&registry),
#[cfg(not(feature = "duckdb"))]
Some(AboutAction::Code) => {
    anyhow::bail!("about code requires the duckdb feature");
}
```

Without the `duckdb` feature the user sees a successful parse, then a runtime anyhow error — the CLI surface advertises a feature the binary cannot deliver.

**Why it matters**: Same shape as the already-filed `Tools` finding (TASK-1319) and the unconditional `AboutAction::Code` variant in `args.rs:158-159`. The two correct shapes are (a) gate the enum variant under `#[cfg(feature = "duckdb")]` so help, tab completion, and parse all reflect the actual capability, or (b) keep the variant unconditional but render the subcommand `hide`d / disabled at help time. Either makes the CLI surface match the binary's actual capability; the current "parse-then-bail" shape is the documented anti-pattern API-1 calls out.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either gate AboutAction::Code under #[cfg(feature = "duckdb")] in args.rs so help/parse reflects the binary's capability, OR hide the subcommand at help-rendering time when the feature is off
- [ ] #2 Resolve in lock-step with TASK-1319 (Tools subcommand has the identical shape)
- [ ] #3 Add a help-rendering test that fails when 'about code' appears in help under a no-duckdb build
<!-- AC:END -->
