---
id: TASK-1845
title: >-
  TEST-5: the deps command path is untested end to end — run_deps, ensure_tools
  and DepsProvider::provide have no test anywhere in the workspace
status: Triage
assignee: []
created_date: '2026-08-27 15:24'
updated_date: '2026-08-27 15:26'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/deps/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:151-187` (`run_deps`), `:113-118` (`ensure_tools`), `:292-305` (`DepsProvider::provide`)

**What**: a workspace-wide grep for `run_deps`, `ensure_tools`, and `DepsProvider` finds no test that calls any of them. `src/tests.rs` covers `build_user_context`, `has_issues` (11 tests), `DepsProvider.schema()`, `check_tool_in`, and the `DataProviderError` source chain — every *leaf*, and none of the wiring between them. The three functions that actually constitute the command are uncovered:

- **`run_deps`** — the whole `ops deps` entry point. Untested: that `opts.refresh` reaches `ctx.refresh`; that the theme/columns are resolved before `get_or_provide` borrows `ctx` mutably (a documented ordering constraint at `:167-173` that only the borrow checker enforces today); that the rendered report reaches stdout; and that `has_issues(&report)` becomes the non-zero exit. That last one is the product's contract — "`ops deps` fails CI when there are dependency issues" — and nothing asserts it. The nearest coverage, `format/render_tests.rs`, renders through the theme "mirroring `run_deps`" by hand-copied duplicate rather than by calling it.
- **`ensure_tools`** — untested. `check_tool_in` is tested for the timeout path only; the loop over `REQUIRED_CARGO_TOOLS`, the not-installed message ("Install with: cargo install cargo-edit"), and the `RunError::Io` arm are not. `ensure_tools` also calls `check_tool`, which hardcodes `Path::new(".")` (`:79-81`) rather than the context's working directory — a divergence no test would catch.
- **`DepsProvider::provide`** — untested. The provider is the only thing that assembles `categorize_upgrades(run_cargo_upgrade_dry_run(..))` + `run_cargo_deny(..)` into a `DepsReport` and serializes it. Whether the two `.context()` labels ("cargo upgrade failed" / "cargo deny failed") are attached to the right calls, and whether the produced JSON round-trips back through the `serde_json::from_value` in `run_deps`, is asserted nowhere. `types/tests.rs::deps_report_serialization_round_trip` builds a `DepsReport` by hand, so it cannot catch a provider that emits a shape `run_deps` then fails to decode.

TEST-31 applies on top: `ops deps` is a CLI subcommand (`crates/cli/src/main.rs:286`) whose exit code and stdout/stderr routing are part of its interface, and no test runs it as a command.

This is testable without the real cargo binaries. `ops-core`'s `test-support` feature is already a dev-dependency, `check_tool_in_times_out_on_hung_probe` demonstrates the fake-`$CARGO`-on-PATH technique for driving the subprocess layer, and `run_deps` takes the `DataRegistry` as a parameter specifically so a stub provider can be registered under `DATA_PROVIDER_NAME`.

Note TASK-1827 asks for one narrow `run_deps` test (stale cached payload → error naming `--refresh`); this finding is the broader gap — the command has no test at all, of which that is one case.

**Why it matters**: every hardening task in this crate's history (TASK-0386, 0598, 0601, 0612, 0913, 0958, 1074, 1202) protects the same property: `ops deps` must fail loudly rather than score green. All of them are pinned at the parser level. The one place where "fails loudly" actually becomes a non-zero exit — `has_issues` → `bail!` inside `run_deps` — is the one place with no test, so a refactor that renders the report and returns `Ok(())` regardless would pass the entire suite.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test drives run_deps with a DataRegistry carrying a stub provider that yields a clean DepsReport, and asserts it returns Ok
- [ ] #2 A test drives run_deps with a stub provider yielding a report containing an actionable advisory, and asserts it returns Err so the non-zero exit contract is pinned
- [ ] #3 A test asserts opts.refresh propagates to ctx.refresh and reaches the provider lookup
- [ ] #4 A test covers DepsProvider::provide producing JSON that run_deps can deserialize back into a DepsReport, closing the provider-to-consumer round trip
- [ ] #5 A test covers ensure_tools reporting a missing tool with the "cargo install <crate>" hint, driven through a fake CARGO env var as check_tool_in_times_out_on_hung_probe already does
- [ ] #6 The tests do not require real cargo-edit or cargo-deny installations and do not reach the network
<!-- AC:END -->
