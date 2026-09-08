---
id: TASK-2230
title: >-
  READ-12: one malformed package.json emits two divergent 'failed to parse
  package.json' warns per ops about run
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-08 07:23'
updated_date: '2026-09-08 11:02'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-node/about/src/package_json.rs
  - extensions-node/about/src/units.rs
priority: low
ordinal: 136000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:95`, `extensions-node/about/src/units.rs:122`

**What**: Both providers registered by this extension parse the *same*
`package.json` (they share the read via `ops_about::manifest_cache` but each
deserialises its own projection), and both log the identical message on a
parse failure:

```rust
// package_json.rs:95 — identity provider
tracing::warn!(path = ?path.display(), error = %e, recovery = "default-identity",
               "failed to parse package.json");

// units.rs:122 — units provider
tracing::warn!(path = ?pkg_path.display(), error = ?e,
               "failed to parse package.json");
```

One `ops about` invocation on a project with a syntax error in `package.json`
therefore emits two `WARN` records with the same message and the same path,
differing only in incidental detail: `%e` vs `?e` for the error, and a
`recovery` field present on one and absent on the other. The `?e` site renders
`serde_json::Error`'s `Debug` form, which is noticeably less readable than the
`Display` form the sibling uses.

Note also that `package_json.rs:90` builds `path` unconditionally on every
call although it is only read inside the error arm.

**Why it matters**: duplicated warnings make an operator hunt for a second
malformed manifest that does not exist, and the divergent field sets mean a
log query filtering on `recovery` or parsing the `error` field sees one of the
two records but not the other.

Related: TASK-2070 (about crate, positional vs named tracing fields).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 a single malformed package.json produces one warn record per ops about run, not one per provider
- [ ] #2 both sites (or the single surviving site) format the serde error consistently and carry the same field set
- [ ] #3 the path value is only constructed on the error path
<!-- AC:END -->
