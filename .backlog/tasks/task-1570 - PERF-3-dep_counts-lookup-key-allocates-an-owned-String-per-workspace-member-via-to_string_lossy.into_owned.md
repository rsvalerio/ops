---
id: TASK-1570
title: >-
  PERF-3: dep_counts lookup key allocates an owned String per workspace member
  via to_string_lossy().into_owned()
status: Done
assignee:
  - TASK-1578
created_date: '2026-05-19 16:35'
updated_date: '2026-05-19 18:48'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:113`

**What**: `let key = canonical_manifest_path.to_string_lossy().into_owned();` allocates a fresh `String` for every workspace member solely to look up `dep_counts.get(&key)`. The `HashMap<String, i64>` key requires a `&str` for lookup (via `Borrow<str>`), so the owned allocation is unnecessary — `canonical_manifest_path.to_str()` (or matching against `Cow::Borrowed`) returns a `&str` borrow without allocation in the common UTF-8 case, and the lookup only needs a borrow.

**Why it matters**: One `String` allocation per workspace member on every `provide()` call, on the same hot path TASK-1201 / TASK-1195 have been tightening. The `into_owned()` also masks the non-UTF-8 case behind a lossy U+FFFD collapse — same sister-policy TASK-0946 / TASK-0986 already mark elsewhere — so a non-UTF-8 canonical path would silently lookup a corrupted key today.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Per-member dep_counts lookup does not allocate an owned String when canonical_manifest_path is valid UTF-8
- [ ] #2 Non-UTF-8 canonical paths are handled explicitly (skip with debug log, or convert via to_str().is_some() guard) rather than lossily collapsed to U+FFFD
- [ ] #3 rust_units_provider_handles_duplicate_named_members test still pins the lookup behavior
<!-- AC:END -->
