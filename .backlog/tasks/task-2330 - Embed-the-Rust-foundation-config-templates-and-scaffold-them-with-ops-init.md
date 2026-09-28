---
id: TASK-2330
title: 'Embed the Rust foundation config templates and scaffold them with ops init'
status: In Progress
assignee: []
created_date: '2026-09-28 13:52'
updated_date: '2026-09-28 14:51'
labels:
  - ci
  - ops-alignment
dependencies: []
modified_files: []
priority: medium
ordinal: 1000
dedup_key: 'ops-align:ops-scaffold'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: the shared Rust config files (clippy.toml, deny.toml, rustfmt.toml, .config/nextest.toml, the [workspace.lints] policy, the test profile) have no single source: forge ships config/* that dbsec vendors via a private forge-sync script, and the ai skills rust-make-clippy-pedantic (apply-config.md) and rust-make-build-fast (apply-templates.md) carry their own divergent copies.

**Decision** (owner, 2026-09-28, recorded on forge TASK-0025): ops is the single source. ops embeds the templates, a scaffold command (e.g. `ops init --stack rust`) writes them into a repo, and a check reports drift from the running ops version's copy. forge's config/ and the ai skills' templates then reference ops.

**Why it matters**: one policy for every Rust repo, updated with ops releases, no vendored copies to sync.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops embeds templates for clippy.toml, deny.toml, rustfmt.toml, .config/nextest.toml and the [workspace.lints] policy, reconciled from forge config/* and the ai skills' templates
- [x] #2 A scaffold command writes them into a Rust repo without overwriting local edits unless asked
- [x] #3 A check reports drift between a repo's files and the running ops version's templates

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented on feat/rust-foundation-templates (owner decisions 2026-09-28: `ops init --rust` / `--rust --check`; semantic TOML baseline with waivers in .ops.toml; lint template at warn).

- New crate extensions-rust/foundation (ops-rust-foundation): embedded templates for clippy.toml, deny.toml, rustfmt.toml, .config/nextest.toml and the lint policy, plus scaffold() and check(). Docs: docs/foundation.md.
- Reconciliation: forge config/* thresholds + ai rust-make-clippy-pedantic allow-*-in-tests + ops's lint policy (warn level, no `cargo` group) + build-fast's nextest leak-timeout + ops's nextest ci JUnit profile. msrv is rendered from rust-version and is not part of the baseline (TASK-2327 covers msrv checks). The test profile is not templated (it is a "do not diverge from dev" rule, not a file).
- Check semantics: baseline keys must match, additions are fine, arrays compare as sets, lint levels only need to be at least as strict, every workspace member needs `[lints] workspace = true`. Waivers: `[foundation.waivers]` in .ops.toml, keyed by location or a file/table prefix; unused waivers are reported.
- ops itself adopted the no-op clippy/deny keys and nextest leak-timeout. The only remaining drift is rustfmt.toml (adopting it reformats about 2600 hunks).

Follow-ups: after release, add `[foundation.waivers] "rustfmt.toml"` to ops's .ops.toml. It cannot land before then, because older ops binaries reject the unknown `[foundation]` section (deny_unknown_fields). Then point forge config/ and the ai skills at ops (forge TASK-0025/0028).
<!-- SECTION:NOTES:END -->
