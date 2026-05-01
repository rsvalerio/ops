---
id: TASK-0796
title: >-
  PERF-1: flatten_coverage_json materialises an intermediate Vec<&Value> only to
  count it for records.reserve
status: Triage
assignee: []
created_date: '2026-05-01 06:00'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:193-202`

**What**: The function builds `let mut files_iter = Vec::new();` and `files_iter.extend(files.iter());` for each data entry, then iterates `for file in &files_iter` — the only reason for the intermediate Vec is `records.reserve(files_iter.len())`.

**Why it matters**: For a typical workspace with thousands of files, this allocates a Vec<&Value> of pointers solely to learn the count, which the JSON has already structured.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Drop files_iter; chain for entry in data { for file in entry[files]...iter() } directly
- [ ] #2 If with_capacity is wanted, compute it from a single sum pass over the data array lengths without materialising a pointer vec
- [ ] #3 Microbench (or visual inspection) confirms no extra allocation per data entry
<!-- AC:END -->
