---
id: TASK-1558
title: >-
  PATTERN-3: flatten_coverage_json dedup does two HashMap probes (get+insert)
  where entry() would suffice
status: Done
assignee:
  - TASK-1577
created_date: '2026-05-19 15:42'
updated_date: '2026-05-19 18:05'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:296-305`

**What**: The dedup hot loop runs `filename_to_idx.get(filename).copied()` followed by `filename_to_idx.insert(filename.to_string(), records.len())` on miss. Two hashes / two probes for every unique filename. Idiomatic Rust uses `entry(filename.to_string()).or_insert_with(...)` or the `raw_entry`/`entry`+match pattern to fuse the lookup and insertion into a single probe.

```rust
match filename_to_idx.get(filename).copied() {
    Some(idx) => {
        duplicate_count += 1;
        records[idx] = record;
    }
    None => {
        filename_to_idx.insert(filename.to_string(), records.len());
        records.push(record);
    }
}
```

**Why it matters**: PATTERN-3 — sibling code in the workspace (e.g. CommandRegistry) uses `entry()`; this module is the odd one out. Two probes per insert is wasted work on every coverage flatten over potentially thousands of files. Replace with `entry(filename.to_string()).or_insert_with(...)` pattern; mark the Occupied branch to bump `duplicate_count` and overwrite `records[idx]`.

<!-- scan confidence: confirmed -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 flatten_coverage_json dedup uses HashMap::entry() instead of get+insert
- [ ] #2 existing dedup tests (flatten_coverage_json_dedups_overlapping_filenames_across_exports, flatten_coverage_json_keeps_distinct_filenames_across_exports) still pass
<!-- AC:END -->
