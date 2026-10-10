---
id: TASK-2465
title: 'READ-13: coverage provider_tests module doc narrates the suite''s own creation'
status: To Do
assignee: []
created_date: '2026-10-10 15:27'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - READ
dependencies: []
parent_task_id: 'TASK-2614'
modified_files:
  - extensions-rust/about/src/coverage_provider.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/about/src/coverage_provider.rs:provider_tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:600-610`

**What**: The `provider_tests` module doc narrates the suite's history: "TEST-5 / TASK-2154: happy-path coverage for `per_crate_units` ... **Before this module the provider was driven by exactly one test** — the non-UTF-8-root skip branch — so the row->unit mapping, the project total, and the default arms of `provide` **had no test at all**." The useful content is what the suite covers and the cross-stack consistency note; the "before this module" sentence is a process artifact of TASK-2154.

**Why it matters**: READ-13: the module's pre-existence state is meaningless to a reader using the tests and goes stale as soon as the suite changes; TASK-2426 cleaned `coverage_provider.rs` but its scope was the `:350` region, so this later-added module doc kept its creation story.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 provider_tests module doc states what the suite covers (row-to-unit mapping, project total, default arms, cross-stack parity with extensions-go/about) without the 'before this module / had no test at all' narration
<!-- AC:END -->
