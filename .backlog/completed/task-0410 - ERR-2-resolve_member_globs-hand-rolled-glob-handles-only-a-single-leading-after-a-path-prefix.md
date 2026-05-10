---
id: TASK-0410
title: >-
  ERR-2: resolve_member_globs hand-rolled glob handles only a single leading *
  after a path prefix
status: Done
assignee:
  - TASK-0417
created_date: '2026-04-26 09:53'
updated_date: '2026-04-27 19:51'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:7-31`

**What**: `resolve_member_globs` is a hand-rolled glob expander that handles exactly one pattern shape: a literal prefix followed by `*` and nothing else (e.g. `crates/*`). It splits on the *first* `*` and reads `parent = workspace_root.join(prefix)`, then enumerates `read_dir(parent)`. Supported by Cargo but unsupported here:

- `crates/*/sub` (the `/sub` suffix after `*` is dropped — directories are matched, not the `sub` child)
- `**/Cargo.toml` style globstar (treated like the bare `*` case, walks only one level)
- Multi-`*` patterns like `vendor/*-plugin/*`
- `?` and `[...]` character-class patterns (Cargo accepts these)

The function does not error on unsupported shapes — it silently produces a wrong member list, which then flows into identity, units, coverage, and dependency providers.

**Why it matters**: A workspace whose `members` uses any of the above shapes shows wrong module counts and missing units in `ops about`. Compounds with TASK-0375 (the duplicate copies of this logic): each copy has the same limitation, so fixing it once requires fixing all three.

**Suggested**: replace the hand-rolled scan with the `glob` or `globwalk` crate, or at least bail visibly on patterns the implementation does not support.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 resolve_member_globs uses a real glob implementation or returns an error for unsupported pattern shapes
- [ ] #2 Test covers crates/*/sub or equivalent suffix-after-star case and asserts correct behavior or visible error
<!-- AC:END -->
