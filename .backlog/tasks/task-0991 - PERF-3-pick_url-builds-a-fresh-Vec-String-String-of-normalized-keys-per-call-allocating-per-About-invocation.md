---
id: TASK-0991
title: >-
  PERF-3: pick_url builds a fresh Vec<(String, &String)> of normalized keys per
  call, allocating per About invocation
status: Done
assignee: []
created_date: '2026-05-04 21:59'
updated_date: '2026-05-04 23:18'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:274-292`

**What**: Each call to `pick_url`:
1. Allocates a `Vec<(String, &String)>` sized to `urls.len()` and fills it by calling `normalize_url_key` (`trim().to_ascii_lowercase().replace('-', " ")` — three String allocations per key) on every URL key.
2. Iterates `keys` (typically 4-6 candidates) and calls `normalize_url_key(target)` again per candidate (one more String allocation per target).
3. On a hit, clones the value String.

`extract_urls` calls `pick_url` twice (for homepage and repository), so for a typical pyproject with N=5 URLs:
- 5 normalised-key allocations × 2 calls = 10
- 5 + 6 candidate-norm allocations = 11
- ~21 short-lived `String` allocs per About run, plus the Vec.

**Why it matters**: Not a hot-loop bug, but the routine is on every `ops about` invocation and the `pick_url` shape was already flagged once (TASK-0685 READ-2 / TASK-0964 ERR-2). A simpler refactor: build the normalised-key map once in `extract_urls` (`HashMap<String, String>` keyed by normalised key, value by reference into the original BTreeMap) and pass it into both `pick_url` calls. That removes the duplicate normalisation work without changing the call site shape.

**Why now**: TASK-0967 (PERF-3 Variables::from_env clones cached strings) and TASK-0968 (PERF-3 prepare_per_crate intermediate Vec) are recent sister findings — keeping the pattern consistent across the codebase.

<!-- scan confidence: candidates to inspect — counted 3 allocs per normalize_url_key call by eye, may be 2 if to_ascii_lowercase short-circuits on already-lowercase input -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_urls normalises each URL key once per About call, not once per pick_url call
- [ ] #2 Existing tests (case-insensitive, kebab-equivalent, precedence) still pass
<!-- AC:END -->
