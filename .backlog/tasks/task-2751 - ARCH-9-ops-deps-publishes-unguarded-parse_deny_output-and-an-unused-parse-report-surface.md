---
id: TASK-2751
title: 'ARCH-9: ops-deps publishes unguarded parse_deny_output and an unused parse/report surface'
status: Done
assignee: []
created_date: '2026-11-18 18:40'
updated_date: '2026-11-18 22:41'
labels:
  - code-review-rust
  - ARCH
dependencies: []
parent_task_id: 'TASK-2851'
modified_files:
  - extensions-rust/deps/src/lib.rs
  - extensions-rust/deps/src/parse/mod.rs
  - extensions-rust/deps/src/parse/deny.rs
priority: medium
ordinal: 1000
dedup_key: 'ARCH-9:extensions-rust/deps/src/lib.rs:pub use parse re-exports'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:61`, `extensions-rust/deps/src/parse/mod.rs:10`, `extensions-rust/deps/src/parse/deny.rs:493`

**What**: The crate root re-exports `parse_deny_output`, which returns `parse_deny_output_inner(stderr).0` and so discards `DenyParseDiagnostics`. It skips `check_partial_decode_loss`, the zero-envelope guard and the configuration-error guard that `interpret_deny_result` applies. The comment above the same `pub use` (lib.rs:53-60) and parse/mod.rs:14-18 say the unguarded upgrade table slicer is kept crate-private for exactly this reason: publishing a slicer that drops its diagnostics "would make opting out of the crate's fail-closed posture the path of least resistance". `parse_deny_output` is the cargo-deny equivalent, it is public, and only the crate's own tests call it (parse/deny/tests.rs:409, 422, 436).

More broadly, no other workspace crate uses most of the public surface. A workspace grep for `ops_deps::` finds only `run_deps`, `DepsOptions` (crates/cli/src/subcommands.rs) and `external_tools` (crates/cli/src/run_cmd/tools.rs). `categorize_upgrades`, `interpret_*`, `run_cargo_*`, `build_report`, `ensure_tools`, `build_user_context` and `DepsProvider` are `pub` with no external caller.

**Why it matters**: A future caller that reaches for `parse_deny_output` gets a parse in which a whole cargo-deny diagnostic class can disappear (schema drift) while it still reports "no advisories", which is the silent supply-chain gate pass the crate's guards exist to prevent. Every extra `pub` item is also API surface that later changes have to preserve.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_deny_output is no longer reachable from outside the crate (crate-private or #[cfg(test)], like parse_upgrade_table), and the in-crate tests still compile
- [x] #2 Each remaining pub re-export from ops-deps either has a caller outside the crate or a documented reason to stay public; the rest become pub(crate)
- [x] #3 cargo build --workspace and the ops-deps tests pass

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.15 (code-review-rust run, 2026-11-18).

Remaining pub surface: run_deps, DepsOptions, external_tools/ExternalTool (CLI callers) plus the extension identity consts and DepsExtension (convention shared by every extension crate, documented at the crate root). Parsers, build_report, types, ensure_tools, build_user_context, DepsProvider are crate-private; parse_deny_output is #[cfg(test)].

<!-- SECTION:NOTES:END -->
