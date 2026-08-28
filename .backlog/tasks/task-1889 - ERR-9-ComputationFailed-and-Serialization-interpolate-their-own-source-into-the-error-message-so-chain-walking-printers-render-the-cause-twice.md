---
id: TASK-1889
title: >-
  ERR-9: ComputationFailed and Serialization interpolate their own #[source]
  into the #[error] message, so chain-walking printers render the cause twice
status: To Do
assignee:
  - TASK-1985
created_date: '2026-08-27 15:34'
updated_date: '2026-08-28 14:10'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - crates/extension/src/error.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/error.rs:87-96`

**What**: both wrapping variants set their message *and* their source to the same value:

```rust
#[error("data computation failed: {0:#}")]
ComputationFailed(#[source] SharedError),

#[error("data serialization error: {0:#}")]
Serialization(#[source] SharedError),
```

`{0:#}` renders the whole chain (`SharedError`'s Display walks `source()` under the alternate flag), and `#[source]` then exposes that same chain again to every printer that walks it. So `anyhow`'s `{:?}` on a `DataProviderError` produces the flattened message followed by a `Caused by:` block repeating each link, and any `tracing` error layer or `eyre`-style reporter does the same. The crate's own test pins the flattened form: `data computation failed: outer context: root cause`, with `outer context` and `root cause` also reachable via `source()`.

This is the exact shape ERR-9 names. It differs from the textbook case in one way worth recording: the interpolation is `{0:#}` rather than `{0}`, so the duplication is the *entire chain* rather than one link.

**Severity note**: filed **Low**, not High. The rationale is documented in detail at `error.rs:79-86` and `error.rs:15-25`, it is specific and accurate (thiserror's nested display does not propagate the alternate flag, so without `{0:#}` a caller formatting with `{e:#}` lost everything past the outermost context), and it names the trade-off explicitly — "duplication is cosmetic, lost root causes are not". Per the skill's justified-violations rule that earns a one-level reduction. This task exists to record the violation and to ask whether a cleaner shape is now available, not to assert the current code is wrong.

**Why it matters**: the workaround was adopted because `SharedError` did not honour the alternate flag. It does now (`error.rs:26-32`), which may have changed what the variant format string needs to do. Worth re-deriving from scratch: with an alternate-aware `SharedError`, does `#[error("data computation failed")]` plus `#[source]` give operators the full chain through the printers this codebase actually uses? If it does, the standard shape wins and the duplication goes away. If it does not — because some caller formats with plain `{e}` and never walks the chain — that is the finding's real answer and belongs in the rustdoc as a named caller, not as a general claim.

**Suggested fix**: enumerate the display paths `DataProviderError` actually reaches (grep the CLI and extension crates for `{e}`, `{e:#}`, `{e:?}`, and `tracing::error!(error = ...)` on this type), pick the format string that serves them with no repetition, and pin the chosen rendering for each path in a test. If `{0:#}` survives the analysis, add a short note to the variant docs saying which printer forced it, so the next reviewer does not re-open this.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The display paths DataProviderError actually reaches are enumerated across the workspace and recorded in the task or the variant rustdoc
- [ ] #2 The #[error] format strings for ComputationFailed and Serialization are re-derived against those paths, dropping the {0:#} self-interpolation if the chain still reaches operators without it
- [ ] #3 A test pins the rendered output for each display path in use ({e}, {e:#}, {e:?} through anyhow), so a future format-string change cannot silently drop a root cause or reintroduce duplication
<!-- AC:END -->
