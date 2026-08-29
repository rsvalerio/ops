---
id: TASK-2024
title: >-
  ERR-1: SharedError::source() skips its own inner error, so every typed-error
  downcast through DataProviderError misses
status: Triage
assignee: []
created_date: '2026-08-28 20:07'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - crates/extension/src/error.rs
  - extensions-rust/about/src/manifest.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/error.rs:37-41` (`impl std::error::Error for SharedError`)

**What**: `SharedError` wraps `Arc<dyn Error + Send + Sync>` but implements

```rust
fn source(&self) -> Option<&(dyn Error + 'static)> { self.0.source() }
```

It returns the source *of* the wrapped error instead of the wrapped error itself, so the inner error is skipped entirely by any `std::error::Error` chain walk. `DataProviderError::ComputationFailed(SharedError)` therefore has a chain of `DataProviderError -> <whatever the original error's own source was>`; the original error object is never a link.

Consequence at a concrete call site: `extensions-rust/about/src/manifest.rs::is_manifest_missing` (moved from the former `query.rs`) walks the chain looking for `FindWorkspaceRootError` or `std::io::Error` in order to distinguish "no Cargo.toml / not a Rust project" from a real read/parse failure. `load_workspace_manifest` builds its error as `DataProviderError::from(anyhow::Error::from(FindWorkspaceRootError::NotFound))`, and the walk never sees the typed marker — `is_manifest_missing` returns `false` and `log_manifest_load_failure` emits `tracing::warn!("failed to load workspace Cargo.toml: …")` for a directory that simply is not a Rust project.

That is exactly the classification ARCH-2 / TASK-0871 and TASK-0433 were built to provide, silently inert. Any other caller doing `err.source().and_then(|s| s.downcast_ref::<T>())` on a `DataProviderError` is affected the same way; note TASK-1887 hardened one such downcast against a *false* positive while this defect produces false negatives everywhere.

`SharedError`'s alternate `Display` (`{:#}`) has the mirror-image shape and is correct there — it prints `self.0` first and *then* walks `self.0.source()` — which is why the message text still reads correctly and hides the broken chain.

**Why it matters**: a cross-crate correctness defect in the error plumbing, not a cosmetic one: it turns typed-error classification into dead code for every consumer of `DataProviderError`, and the visible symptom (a warn on every non-Rust directory) looks like a bug in the about providers rather than in the error type.

**Origin**: discovered during TASK-1993 (code-review-plan-wave159) while fixing TASK-1791/TASK-1762 — a new test asserting `is_manifest_missing` on a real missing-manifest error failed. The fix is out of the wave's file scope (`crates/extension`) and has cross-crate blast radius, so it is filed rather than applied; `manifest.rs`'s test documents the behaviour and deliberately does not pin it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 SharedError::source() yields the wrapped error itself, so a chain walk over DataProviderError reaches the originating typed error
- [ ] #2 A regression test constructs DataProviderError::from(anyhow::Error::from(<typed error>)) and asserts the typed error is reachable via downcast_ref through the source chain
- [ ] #3 extensions-rust/about is_manifest_missing classifies a missing workspace Cargo.toml as not-found, and log_manifest_load_failure emits debug rather than warn for it (pinned by a test)
- [ ] #4 SharedError's alternate Display output is unchanged - no link is printed twice or dropped by the source() fix
<!-- AC:END -->
