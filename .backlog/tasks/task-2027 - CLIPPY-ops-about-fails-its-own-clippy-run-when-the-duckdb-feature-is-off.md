---
id: TASK-2027
title: 'CLIPPY: ops-about fails its own clippy run when the duckdb feature is off'
status: Triage
assignee: []
created_date: '2026-08-28 20:25'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/about/src/units.rs
  - extensions/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:183`, `extensions/about/src/lib.rs:234`

**What**: `cargo clippy -p ops-about --all-targets` (no `--all-features`) fails
with two `clippy::missing_const_for_fn` errors on the no-op `enrich_from_db`
stubs that are compiled when the `duckdb` feature is off:

```
error: this could be a `const fn`
   --> extensions/about/src/units.rs:183:1
    | fn enrich_from_db(_ctx: &Context, _units: &mut [ProjectUnit]) {}
error: this could be a `const fn`
   --> extensions/about/src/lib.rs:234:1
    | fn enrich_from_db(_ctx: &ops_extension::Context, _identity: &mut ProjectIdentity) {}
```

The workspace gate (`ops verify`, which runs `--all-features`) never sees this:
with `duckdb` on, the real implementations replace the stubs. Only a per-crate
or feature-restricted invocation hits it.

**Why it matters**: `-D clippy::nursery` is workspace policy, so any developer
or CI step that lints a subset of crates (`cargo clippy -p <crate>`) gets a red
build from code they did not touch. The fix is one word per stub (`const fn`)
or a narrow `#[allow]` with a reason, per `docs/clippy.md`.

**Origin**: discovered during TASK-1995 while fixing TASK-1794 (adding
`ops-about` with the `test-support` feature as a dev-dependency put the
feature-off build of `ops-about` into a per-crate lint run).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cargo clippy -p ops-about --all-targets passes with no extra flags and the duckdb feature off
- [ ] #2 The fix is applied at the narrowest scope (const fn, or an allow carrying its reason) per docs/clippy.md
<!-- AC:END -->
