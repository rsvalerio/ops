---
id: TASK-2070
title: 'READ-12: tracing warns interpolate values positionally instead of named structured fields'
status: Done
assignee: []
created_date: '2026-09-07 22:55'
updated_date: '2026-09-10 16:19'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions/about/src/lib.rs
  - extensions/about/src/code.rs
  - extensions/about/src/loc.rs
  - extensions/about/src/providers.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:156,201,208,215,224,230`; `extensions/about/src/code.rs:33`; `extensions/about/src/loc.rs:76,87`; `extensions/about/src/providers.rs:73`

**What**: <!-- scan confidence: candidates to inspect --> Ten `tracing::warn!` call sites bake values into the message string with positional `{}`/`{e:#}` formatting, e.g. `tracing::warn!("about: query_project_loc failed: {e:#}")` and `tracing::warn!("about/{subpage}: warm-up {provider} failed: {e:#}")`. The rest of this same crate already uses the named-field form (`workspace.rs`, `manifest_io.rs`, `manifest_cache.rs`, `units.rs` use `error = ?e`, `parent = ?parent.display()` etc.), so these ten sites are the drift.

**Why it matters**: READ-12 — a value embedded in the message text cannot be filtered, grouped, or aggregated on by a subscriber/collector the way a named field can. It is also a READ-6 consistency issue: two logging dialects in one crate means the next warn call site copies whichever one is nearest.

Non-test candidates (all verified non-`#[cfg(test)]`):
- lib.rs:156 `coverage collection failed: {e:#}`
- lib.rs:201 `about: query_project_loc failed: {e:#}`
- lib.rs:208 `about: query_project_file_count failed: {e:#}`
- lib.rs:215 `about: query_dependency_count failed: {e:#}`
- lib.rs:224 `about: query_project_coverage failed: {e:#}`
- lib.rs:230 `about: query_project_languages failed: {e:#}`
- code.rs:33 `language_stats: query_project_languages failed: {e:#}`
- loc.rs:76 `about/loc: query_rust_loc_summary failed: {e:#}`
- loc.rs:87 `about/loc: query_rust_loc_file_count failed: {e:#}`
- providers.rs:73 `about/{subpage}: warm-up {provider} failed: {e:#}`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every listed warn site uses named fields (e.g. error = ?e, provider, subpage) with a stable message template
- [x] #2 No positional value interpolation remains in tracing calls in extensions/about non-test code

<!-- AC:END -->
