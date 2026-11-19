---
id: TASK-2869
title: 'API-14: missing_docs is enforced nowhere, so public-item doc gaps regress silently'
status: Triage
assignee: []
created_date: '2026-11-18 21:42'
labels:
  - code-review-rust
  - API
dependencies: []
modified_files:
  - Cargo.toml
  - extensions/text-fixers/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `Cargo.toml` (`[workspace.lints.rust]`), `extensions/text-fixers/src/lib.rs`

**What**: No crate in the workspace runs `missing_docs`: TASK-2779 removed the only crate-root `#![warn(missing_docs)]` (config-checkers) to keep lint policy in `[workspace.lints]`, and declined adding it there because ~30 crates were never held to it. TASK-2827 asked for the same crate-root attribute in ops-text-fixers; it was not added for the same reason. ops-text-fixers and ops-config-checkers currently build clean under `-D missing_docs`, but nothing keeps them so.

**Why it matters**: The API-14 gaps fixed in TASK-2827 were only found by review; the next undocumented public item in these crates will be too.

**Origin**: discovered during TASK-2859 while fixing TASK-2827.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A workspace-level decision is recorded: missing_docs added to [workspace.lints.rust] at a gating level (with the undocumented items fixed or the opted-out crates listed), or a documented exemption mechanism lets doc-complete crates such as ops-text-fixers and ops-config-checkers opt in
<!-- AC:END -->
