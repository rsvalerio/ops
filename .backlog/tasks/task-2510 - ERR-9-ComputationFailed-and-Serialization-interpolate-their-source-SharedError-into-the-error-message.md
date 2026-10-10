---
id: TASK-2510
title: 'ERR-9: ComputationFailed and Serialization interpolate their #[source] SharedError into the error message'
status: To Do
assignee: []
created_date: '2026-10-10 15:32'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - err
dependencies: []
parent_task_id: 'TASK-2612'
modified_files:
  - crates/extension/src/error.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-9:crates/extension/src/error.rs:DataProviderError'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/error.rs:177` (ComputationFailed), `crates/extension/src/error.rs:201` (Serialization)

**What**: Both variants set the message and the source to the same value: `#[error("data computation failed: {0:#}")] ComputationFailed(#[source] SharedError)` and `#[error("data serialization error: {0:#}")] Serialization(#[source] SharedError)`. Chain-walking printers therefore render each link twice: anyhow's `{:?}` shows the message followed by a `Caused by:` block repeating the same chain.

**Why it matters**: ERR-9 flags any `#[error]` whose format argument is the field marked `#[from]`/`#[source]`. Here the violation carries an extensive in-code justification (`# Why the message interpolates its own #[source]` on ComputationFailed): thiserror does not propagate the alternate flag through nested `{0}`, so dropping the `#` would lose everything past the outermost context in plain `{e}` / `to_string()` / `tracing::warn!` output — the paths this error predominantly reaches operators through. The rendering of all three display paths is pinned by tests (`computation_failed_rendering_is_pinned_on_every_display_path`, `serialization_rendering_flattens_its_chain`). Per the rules-classification note on justified violations (specific, accurate, test-pinned rationale), severity is downgraded one level from the ERR baseline to Low.

**Residual cost**: duplicate chain links in anyhow `{:?}` debug reports (accepted and pinned as deliberate today).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either the #[error] format strings stop interpolating the #[source] field while every existing rendering test stays green (e.g. a hand-written Display for the variants that walks the SharedError chain, so plain {} keeps root causes without source interpolation), or the task is closed with the documented trade-off explicitly re-accepted
<!-- AC:END -->
