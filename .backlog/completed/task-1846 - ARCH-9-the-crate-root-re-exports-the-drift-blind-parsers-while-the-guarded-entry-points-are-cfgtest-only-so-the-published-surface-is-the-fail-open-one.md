---
id: TASK-1846
title: >-
  ARCH-9: the crate root re-exports the drift-blind parsers while the guarded
  entry points are cfg(test)-only, so the published surface is the fail-open one
status: Done
assignee:
  - TASK-1997
created_date: '2026-08-27 15:24'
updated_date: '2026-08-28 20:41'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-rust/deps/src/lib.rs
  - extensions-rust/deps/src/parse/mod.rs
  - extensions-rust/deps/src/parse/upgrade.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:33-38`, `extensions-rust/deps/src/parse/mod.rs:10-16`, `extensions-rust/deps/src/parse/upgrade.rs:116-119`

**What**: the crate root publishes this surface:

```rust
pub use format::build_report;
pub use parse::{
    categorize_upgrades, interpret_deny_result, parse_deny_output, parse_upgrade_table,
    run_cargo_deny, run_cargo_upgrade_dry_run,
};
pub use types::*;
```

while `parse/mod.rs` gates the *safe* counterpart behind `cfg(test)`:

```rust
#[cfg(test)]
pub use upgrade::interpret_upgrade_output;
```

Three problems follow from that split:

1. **`parse_upgrade_table` is a public function with zero production callers.** A workspace-wide grep finds it used only from `parse/upgrade/table_tests.rs` and `exit_code_tests.rs`; the real path is `run_cargo_upgrade_dry_run` → `interpret_upgrade_output` → `parse_upgrade_table_inner`. `parse_upgrade_table` exists solely as `parse_upgrade_table_inner(stdout).0` — it *discards* `UpgradeParseDiagnostics`, which means it bypasses `check_header_drift` (TASK-1074), `check_row_shape_drift` (TASK-1202), and the missing-separator case (TASK-1817). It is precisely the drift-blind shortcut the crate spent four tasks hardening against, exported at the root with a doc comment that never says so. Any future caller reaching for the obvious-looking `ops_deps::parse_upgrade_table` silently opts out of every guard.

2. **The guarded counterpart is not reachable outside tests.** `interpret_upgrade_output` — the function that carries the drift contract — is `#[cfg(test)] pub use`. So a caller who wants the safe behaviour and does not want to spawn a subprocess has nothing to call. (`interpret_deny_result` is exported unconditionally, so the deny side is already the right shape; the upgrade side is the asymmetry.)

3. **`pub use types::*`** makes the public surface implicit: every type added to `types.rs` is published automatically, including ones intended as internals. ARCH-4 asks for curated re-exports in `lib.rs`; a glob is the opposite.

The fix is to decide which of these are API and which are implementation. `parse_upgrade_table` should either become private (`pub(crate)`, or deleted with its tests moved onto `parse_upgrade_table_inner` / `interpret_upgrade_output`) or, if a diagnostics-free parse is genuinely wanted, keep the name but return the diagnostics so the caller cannot ignore drift by accident. `interpret_upgrade_output` should be exported on the same terms as `interpret_deny_result`. `pub use types::*` should be an explicit list.

**Why it matters**: this is not a hypothetical. `extensions-rust/about` already builds its own `RustDepsProvider` over similar ground, and the crate is a shared workspace library — the next consumer picks a function by name from the root re-export list. Right now the name that reads like "parse the upgrade table" is the one that cannot fail and cannot warn, and the name that enforces the crate's whole fail-closed posture is invisible outside `cfg(test)`. A safety property that depends on callers choosing the less obvious function is a fragile API (design philosophy: "if documentation is required to prevent misuse, the API is fragile").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_upgrade_table is no longer a root-level public function that silently discards UpgradeParseDiagnostics — it is made crate-private, removed, or changed to hand the diagnostics back to the caller
- [x] #2 interpret_upgrade_output is exported on the same terms as interpret_deny_result rather than only under cfg(test)
- [x] #3 pub use types::* is replaced by an explicit re-export list so adding a type to types.rs is not automatically a public API change
- [x] #4 Existing table_tests.rs and exit_code_tests.rs keep their coverage, calling whatever the guarded entry point becomes
- [x] #5 The crate still compiles with no unused-import or dead-code warnings and the workspace clippy lints stay clean
<!-- AC:END -->
