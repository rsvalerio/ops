# Implementation Plan: 39 Backlog Tasks

## Execution Progress

Last updated: 2026-04-15

| Wave | Status | Notes |
|------|--------|-------|
| **Wave 0** | **DONE** | RS-1, RS-3, RS-4, TQ-1 all complete |
| **Wave 1** | **DONE** | CD-3, CD-5, CD-6, CD-7, task-0024 all complete |
| **Wave 2** | **DONE** | CQ-8, CQ-10, CQ-11, task-0023 all complete |
| **Wave 3** | **DONE** | CQ-7, CD-1, CD-4 all complete |
| **Wave 4** | **DONE** | CQ-6, CQ-9, CD-2, CD-9, CD-10 all complete |
| **Wave 5** | **DONE** | CD-13, CQ-1, CQ-13, CD-8, CQ-4, CD-12 all complete |
| **Wave 6** | **DONE** | CQ-2, CQ-3, CQ-5, CD-11, RS-2 complete; CQ-12 already resolved in prior wave |
| **Wave 7** | **DONE** | TQ-5, TQ-7, TQ-4, TQ-2, TQ-3, TQ-6 all complete |

**Note**: Many tasks from Waves 0–3 were partially completed in a prior session (unstaged changes). The remaining work was finished and verified (fmt, clippy, test, install) on 2026-04-15. All changes are currently **unstaged** — no commits have been made yet.

---

## Context

The `.backlog/tasks/` directory contains 39 open tasks across 4 categories:
- **CD** (Code Duplication): 15 tasks — extract shared utilities, deduplicate patterns
- **CQ** (Code Quality): 13 tasks — split god modules, reduce function length/nesting
- **RS** (Security): 4 tasks — SQL injection defense, safe casts, dependency advisory
- **TQ** (Test Quality): 7 tasks — fill coverage gaps for critical code paths

These tasks are ordered into 8 waves based on dependency chains. The critical path is: RS-1 → CQ-7 → CD-1 (security fix enables god-module split enables query dedup).

---

## Wave 0: Security Foundations

**Why first**: RS-1 is a prerequisite for the largest refactoring chain, and security fixes are non-negotiable.

| Task | Files | Change |
|------|-------|--------|
| **RS-1** (task-0036) | `extensions/duckdb/src/sql.rs` | Add `validate_table_name()` whitelisting `[a-zA-Z_][a-zA-Z0-9_]*` |
| **RS-3** (task-0044) | `extensions/duckdb/src/ingestor.rs:82` | Replace `as u64` with `u64::try_from(val).unwrap_or(0)` |
| **RS-4** (task-0058) | `extensions/duckdb/src/sql.rs:434` | Replace `as usize` with `usize::try_from(val).unwrap_or(0)` |
| **TQ-1** (task-0045) | `extensions/duckdb/src/sql.rs` (tests) | Add edge-case tests: unicode, null bytes, `--` comments, `UNION` injection |

**Order**: RS-1 → RS-4 → TQ-1 (same file); RS-3 in parallel (different file).

---

## Wave 1: Shared Utility Foundations

**Why**: Creates reusable utilities that later waves depend on. All independent, all parallel.

| Task | Files | Change |
|------|-------|--------|
| **CD-6** (task-0038) | `crates/core/src/project_identity.rs` | Add `Default` impl for `ProjectIdentity`; simplify 5 provider sites |
| **CD-3** (task-0031) | `extensions/about/src/lib.rs`, `crates/cli/src/extension_cmd.rs` → `crates/core/src/text.rs` | Extract `capitalize()` to core |
| **CD-7** (task-0049) | `crates/core/src/project_identity.rs`, `extensions-rust/about/src/text_util.rs` | Unify `format_number` in core, delete duplicate |
| **CD-5** (task-0037) | `extensions/about/src/lib.rs`, `extensions-rust/about/src/query.rs` → core | Extract `maybe_spinner()` to shared location |
| **task-0024** | 3 about extensions → `crates/core/src/text.rs` | Extract `dir_name()` to core |

---

## Wave 2: Mechanical Structural Moves

**Why**: Reduces file sizes without logic changes. Low risk, high readability payoff.

| Task | Files | Change |
|------|-------|--------|
| **CQ-8** (task-0039) | `extensions-rust/about/src/lib.rs` (1635 lines) | Move ~1588 lines of tests to submodule test files |
| **CQ-10** (task-0041) | `extensions-java/about/src/lib.rs` (998 lines) | Split into `maven.rs` + `gradle.rs` + slim `lib.rs` |
| **CQ-11** (task-0042) | `crates/theme/src/lib.rs` (398 lines) | Split into `trait.rs` + `configurable.rs` + slim `lib.rs` |
| **task-0023** | `extensions/run-before-commit`, `extensions/run-before-push` | Extract shared hook logic; parameterize differences (hook name, env var, staged-files check) |

All parallel (different crates).

---

## Wave 3: DuckDB Module Restructuring (Critical Chain)

**Why**: The longest dependency chain. CQ-7 splits the god module, then CD-1/CD-4 deduplicate within the new structure.

| Task | Prereqs | Change |
|------|---------|--------|
| **CQ-7** (task-0035) | RS-1 | Split `sql.rs` → `sql/validation.rs`, `sql/ingest.rs`, `sql/query.rs`, `sql/mod.rs` (re-exports preserve public API) |
| **CD-1** (task-0029) | CQ-7 | Extract `query_with_table_check()` helper for 7 query functions sharing 65% scaffolding |
| **CD-4** (task-0032) | CQ-7 | Extract `get_db(ctx)` helper for DuckDB downcast pattern repeated 5x across 3 files |

**Order**: CQ-7 first, then CD-1 and CD-4 in parallel.

---

## Wave 4: Data Model & Identity Refactors

**Why**: Depends on CD-6 (Wave 1) and CQ-8 (Wave 2). Simplifies repetitive field-extraction code.

| Task | Prereqs | Change |
|------|---------|--------|
| **CQ-6** (task-0034) | CD-6, CQ-8 | Simplify `RustIdentityProvider::provide` (91 lines) with `resolve_field()` helper |
| **CQ-9** (task-0040) | CD-6 | Data-driven `from_identity_filtered` — field descriptor table instead of 11 repeated blocks |
| **CD-2** (task-0030) | — | Extract 7 common `AboutFieldDef` entries to shared constant; stack providers concatenate common + specific |
| **CD-9** (task-0051) | — | Unify `coverage_icon`/`coverage_color` via `CoverageTier` enum with shared thresholds |
| **CD-10** (task-0052) | — | Merge `LicenseEntry`/`BanEntry`/`SourceEntry` into single `DenyEntry` struct |

CQ-6/CQ-9 sequential after their prereqs; CD-2, CD-9, CD-10 independent and parallel.

---

## Wave 5: CLI & Runner Refactors

**Why**: Higher-traffic entry points; benefits from shared utilities being in place.

| Task | Prereqs | Change |
|------|---------|--------|
| **CD-13** (task-0055) | — | Replace inlined description logic in `main.rs` with `hook_shared::command_description` call |
| **CQ-1** (task-0025) | CD-13 | Split `print_categorized_help` (133 lines) into `collect_entries()`, `sort_and_group()`, `render_help()` |
| **CQ-13** (task-0057) | — | Split `dispatch` (90 lines) — extract inline match arms to handler functions |
| **CD-8** (task-0050) | — | Extract `lookup_in_sources()` for three-tier command lookup repeated in 4 methods |
| **CQ-4** (task-0028) | — | Flatten `on_plan_started` nesting in `crates/runner/src/display.rs` |
| **CD-12** (task-0054) | CQ-11 | Deduplicate `render_summary` — trait default calls shared impl with prefix param |

CD-13 → CQ-1 sequential; all others parallel.

---

## Wave 6: Remaining Refactors

**Why**: Independent medium-complexity tasks with no blockers remaining.

| Task | Change |
|------|--------|
| **CQ-2** (task-0026) | Simplify `query_crate_coverage` (benefits from CD-1 scaffolding helper) |
| **CQ-3** (task-0027) | Split `deps/lib.rs` (726 lines) → `parsing.rs` + `formatting.rs` |
| **CQ-5** (task-0033) | Refactor `parse_pom_xml` — replace boolean-flag state machine |
| **CQ-12** (task-0056) | Refactor `render_detail_section` (109 lines) — extract section-rendering helpers |
| **CD-11** (task-0053) | Unify Stack parallel match arms via `StackMeta` struct |
| **RS-2** (task-0043) | Check if `rand` advisory resolves with `cargo update`; if not, document in `deny.toml` |

All parallel (different files/crates).

---

## Wave 7: Test Coverage

**Why**: Tests written against final refactored code to avoid rewriting.

| Task | Prereqs | Change |
|------|---------|--------|
| **TQ-5** (task-0059) | — | Add unit tests for `config/merge.rs` (4 functions, zero coverage) |
| **TQ-7** (task-0061) | — | Add `collect()`/`load()` tests for MetadataIngestor & CoverageIngestor |
| **TQ-4** (task-0048) | — | Evaluate 7 framework-only Extension trait tests; remove if noise-only |
| **TQ-2** (task-0046) | Wave 3 | Test 4 uncovered `query.rs` functions |
| **TQ-3** (task-0047) | Wave 4 | Test `DataRegistry::about_fields` and `detail_sections` |
| **TQ-6** (task-0060) | Wave 4 | Test `RustIdentityProvider` with mock Cargo.toml data |

TQ-5, TQ-7, TQ-4 can start immediately; TQ-2/3/6 wait for their prereqs.

---

## Verification

After every `.rs` change per AGENTS.md:
```bash
ops verify    # fmt → check → clippy → build
ops qa        # test → deps
ops install   # cargo install --path crates/cli --force
```

Batch changes per-crate within a wave to minimize verification cycles. Each task gets a dedicated commit following conventional commit style.

---

## High-Traffic Files (most tasks touching them)

1. `extensions/duckdb/src/sql.rs` — 6 tasks (RS-1, RS-4, TQ-1, CQ-7, CD-1, CQ-2)
2. `crates/core/src/project_identity.rs` — 4 tasks (CD-6, CD-7, CQ-9, CD-2)
3. `extensions-rust/about/src/identity.rs` — 4 tasks (CQ-6, CD-4, CQ-12, TQ-6)
4. `crates/cli/src/main.rs` — 3 tasks (CD-13, CQ-1, CQ-13)
5. `extensions-rust/about/src/lib.rs` — 2 tasks (CQ-8, CD-5)
