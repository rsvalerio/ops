---
id: TASK-2387
title: 'READ-13: Doc and inline comments narrate design history in ops-about-terraform'
status: Done
assignee: []
created_date: '2026-10-04 14:13'
updated_date: '2026-10-04 16:14'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2424'
modified_files:
  - extensions-terraform/about/src/lib.rs
  - extensions-terraform/about/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-terraform/about/src/lib.rs:module-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->
**File**: `extensions-terraform/about/src/lib.rs` (candidates: 14-18, 141-143, 219-223, 872-874, 1070-1074, 1110-1120; plus dependency comments in `extensions-terraform/about/Cargo.toml`)

**What**: Item docs and inline comments explain how the code got to its current shape ("rather than one loop body mixing all of them", "rather than being dropped through `flatten()` or an `exists()` probe", "`map_or` would bury the common shaped pair in a closure", "probing for `modules/*/main.tf` would report them as zero modules"). Several comments also read as if a task reference was stripped, leaving mid-thought wrapping (for example lines 110-112, 228-232, 256-260, 299-303, 312-314, 496-497, 835-838, 842-844, 905-909, 1127-1129).

**Why it matters**: Text a later reader would delete verbatim goes stale while looking authoritative. Docs should state current behavior and enduring constraints (drop-not-strip policy, line-local strings, IO warn policy), which the module docs already do well.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each listed candidate describes current behavior and invariants only, with contrast-with-alternative and history phrasing removed
- [x] #2 Comments with broken mid-sentence wrapping are reflowed

<!-- AC:END -->
