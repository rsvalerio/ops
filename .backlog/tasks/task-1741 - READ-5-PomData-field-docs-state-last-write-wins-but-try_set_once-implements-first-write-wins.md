---
id: TASK-1741
title: >-
  READ-5: PomData field docs state last-write-wins but try_set_once implements
  first-write-wins
status: Triage
assignee: []
created_date: '2026-08-27 11:13'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-java/about/src/maven/pom.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:33-37`

**What**: The `PomData` field docs assert the opposite of the behaviour the module actually implements:

```rust
/// Maven `<artifactId>` — coordinate, last-write-wins on duplicates.
pub(super) artifact_id: Option<String>,
/// Maven `<name>` — display name, last-write-wins on duplicates.
/// Provider prefers this over `artifact_id` when both are present.
pub(super) name: Option<String>,
```

Every top-level field is written through `try_set_once` (`pom.rs:255`), whose own doc comment says the exact opposite and is the accurate one:

> write `field` from a `<tag>value</tag>` line iff the field is still empty. Encodes the "first writer wins on duplicates" invariant …

Verified empirically against a copy of this module: with two `<artifactId>` elements in a POM, the **first** is kept (the `<parent>` leak in the sibling finding depends on exactly this).

Nothing reads the field docs at compile time, so the contradiction survives — and it is load-bearing: `parse_pom_scm_takes_precedence_over_url` exists specifically to pin the first-writer-wins ordering, so a reader who trusts the struct docs and "fixes" a field to overwrite would break that test with no idea why.

**Why it matters**: READ-5 — the stated invariant is the wrong one, on the two fields that determine the project name shown in `ops about`. A wrong doc is worse than no doc: it actively invites the regression the helper was written to prevent.

**Fix**: correct both field docs to "first-write-wins on duplicates" and point at `try_set_once` as the single place the policy lives.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 PomData::artifact_id and PomData::name docs describe first-write-wins, matching try_set_once
- [ ] #2 The docs reference try_set_once as the single owner of the duplicate-resolution policy
<!-- AC:END -->
