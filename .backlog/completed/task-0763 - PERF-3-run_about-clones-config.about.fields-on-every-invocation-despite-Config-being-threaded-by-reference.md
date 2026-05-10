---
id: TASK-0763
title: >-
  PERF-3: run_about clones config.about.fields on every invocation despite
  Config being threaded by reference
status: Done
assignee:
  - TASK-0828
created_date: '2026-05-01 05:54'
updated_date: '2026-05-02 08:14'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:49`

**What**: `config.about.fields.clone()` — AboutOptions::new takes Option<Vec<String>> by value. CLI threads &Config from run() (TASK-0427), but about path immediately reclones the fields vector.

**Why it matters**: Minor allocation pressure but the clone is gratuitous. AboutOptions could borrow with a lifetime or take a &[String]. Restructures around OWN-8.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AboutOptions (in ops_about) accepts &[String] or Cow<[String]>, OR run_about takes ownership of Config so the clone is replaced by a move
- [ ] #2 cargo expand / clippy redundant_clone confirms no implicit clone remains
- [ ] #3 Change is contained to the about wiring; no behaviour change observable
<!-- AC:END -->
