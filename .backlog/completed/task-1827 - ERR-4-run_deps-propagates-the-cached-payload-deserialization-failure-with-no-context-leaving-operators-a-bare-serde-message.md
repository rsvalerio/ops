---
id: TASK-1827
title: >-
  ERR-4: run_deps propagates the cached-payload deserialization failure with no
  context, leaving operators a bare serde message
status: Done
assignee:
  - TASK-1997
created_date: '2026-08-27 11:34'
updated_date: '2026-08-28 20:29'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/deps/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:175-176`

**What**:

```rust
let value = ctx.get_or_provide(DATA_PROVIDER_NAME, data_registry)?;
let report: DepsReport = serde_json::from_value(std::sync::Arc::unwrap_or_clone(value))?;
```

Neither `?` carries a `.with_context()`. Every other fallible call in this crate attaches context — `build_user_context` uses `.context("deps: failed to determine current working directory")`, `DepsProvider::provide` uses `.context("cargo upgrade failed")` / `.context("cargo deny failed")`, and the theme resolution on the preceding line maps into `anyhow!("deps: {e}")`. These two are the exception.

The deserialization one matters most because its input is a *cached* JSON blob, not freshly-produced data: `get_or_provide` returns a previously persisted payload when one exists. `DepsReport` and its members are `#[non_exhaustive]` and evolving (fields have been added across TASK-0601/0845/1041), so a cache written by an older `ops` is a live failure mode. When it hits, the operator sees only serde's own message — e.g. `missing field \`upgrades\`` or `invalid type: null, expected a sequence` — with no mention of `ops deps`, no mention that the payload came from the data cache rather than from cargo, and no hint that `ops deps --refresh` (which sets `ctx.refresh`, already plumbed through `DepsOptions` two lines above) is the fix.

ERR-14 applies in the same spot: the failing field path is not surfaced either, so on a nested `DepsReport` the message does not say *which* section failed to decode.

**Why it matters**: this is the one error in the command whose remedy the user cannot guess from the message. A bare `missing field \`upgrades\`` on `ops deps` reads as a bug in ops, and the actual one-word fix (`--refresh`) is invisible. The `get_or_provide` line has the same problem in milder form: a provider-registry failure surfaces without naming `deps` as the provider being resolved.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The serde_json::from_value failure in run_deps carries context naming the deps report payload and the fact that it may come from the data cache
- [x] #2 The context (or the error message) points the operator at re-running with --refresh
- [x] #3 ctx.get_or_provide's error carries context naming the deps data provider
- [x] #4 A test asserts that deserializing a stale/incompatible cached payload produces an error whose Display chain mentions both the deps report and --refresh
<!-- AC:END -->
