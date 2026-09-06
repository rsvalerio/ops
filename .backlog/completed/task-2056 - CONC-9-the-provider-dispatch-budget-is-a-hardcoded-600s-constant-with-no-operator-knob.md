---
id: TASK-2056
title: >-
  CONC-9: the provider dispatch budget is a hardcoded 600s constant with no
  operator knob
status: Done
assignee:
  - TASK-2060
created_date: '2026-08-29 13:31'
updated_date: '2026-08-29 18:09'
labels:
  - code-review-rust
  - concurrency
dependencies: []
modified_files:
  - crates/extension/src/data.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs` (`DEFAULT_PROVIDER_BUDGET`)

**What**: TASK-2017 gave provider dispatch a wall-clock bound, but the value is
a `const` compiled into `ops-extension` and the only override is the
programmatic `Context::with_provider_budget`, which nothing in the CLI wiring
calls. An operator on a slow network filesystem cannot tighten it, and an
operator running a genuinely long provider (a full-workspace `cargo llvm-cov`
through the coverage providers) cannot widen it or turn it off; the constant
has to be generous enough for the slowest in-tree provider, which makes it
loose for the ones the SEC-33 finding was actually about.

This is the same shape as TASK-2022 (the post-exit capture drain deadline),
which was filed and fixed for exactly this reason.

**Why it matters**: a timeout nobody can configure is tuned for the worst case
and therefore protects the common case badly. It also means the only way to
diagnose a suspected provider stall is to rebuild with a different constant.

**Origin**: discovered during TASK-2047 while fixing TASK-2017.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The provider dispatch budget is readable from config (with the current constant as the default) rather than only from a const
- [x] #2 The wiring that builds a Context applies the configured value, pinned by a test
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in code-review/TASK-2060.

AC #1: new `[data] provider_budget_secs` key on `DataConfig`
(`crates/core/src/config/sections.rs`), documented in
`crates/core/src/.default.ops.toml`. Unset keeps `DEFAULT_PROVIDER_BUDGET`
(still the compiled-in default, now genuinely a *default* rather than the only
value); any other value replaces it, which is what lets an operator on a slow
network filesystem tighten it.

`0` is the opt-out and maps to `None` (unbounded) rather than to a zero-length
budget — read literally it would expire every dispatch instantly, turning a
knob meant to *remove* the bound into one that fails every provider. That is
the operator running a full-workspace `cargo llvm-cov` through the coverage
providers.

AC #2: the resolution happens in `Context::from_cwd_arc`, which `Context::new`
also routes through, so **every** construction path honours the configured
value without separate wiring — there is no call site left that can forget it.
`Context::provider_budget()` exposes the resolved value. Pinned by four tests
in `crates/extension/src/tests.rs`: unset yields the default, a configured
value replaces it, `0` is unbounded (driven end-to-end through a slow provider
that must still complete), and the programmatic `with_provider_budget`
override still wins over the project file.

Chose a TOML key over TASK-2022's env-var precedent because `from_cwd_arc`
already holds the `Arc<Config>`: the value is per-project (a repo on a slow
mount wants it tightened for everyone), which is what `.ops.toml` is for, and
it needs no new resolver, cache, or clamp.
<!-- SECTION:NOTES:END -->
