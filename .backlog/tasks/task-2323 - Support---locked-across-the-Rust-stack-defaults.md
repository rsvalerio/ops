---
id: TASK-2323
title: 'Support --locked across the Rust stack defaults'
status: Triage
assignee: []
created_date: '2026-09-28 10:59'
labels:
  - ci
  - ops-alignment
dependencies: []
modified_files: []
priority: medium
ordinal: 1000
dedup_key: 'ops-align:ops-locked'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: no stack command passes `--locked` (only clippy-findings does). event0 re-declares 9 built-ins (build, check, clippy, doc, test, test-ignored, next, next-ignored, test-doc) solely to add it.

**Why it matters**: CI must not resolve a different lockfile than the one committed; the fix belongs in the defaults, not in every repo.

**Origin**: ops-alignment survey of forge, ops and ai, 2026-09-28.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rust stack cargo commands run `--locked` by default or via one switch (flag/env/config)
- [ ] #2 event0 can drop its re-declared built-ins (follow-up noted there)
<!-- AC:END -->
