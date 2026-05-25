---
id: TASK-1118
title: 'DUP-3: ''Description'' column position lookup duplicated in extension_cmd.rs'
status: Done
assignee: []
created_date: '2026-05-07 22:07'
updated_date: '2026-05-07 23:30'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:86-89`, `crates/cli/src/extension_cmd.rs:331-334`

**What**: Both `write_extension_table` and `print_provider_info` build a small string-array of headers and then run an identical 4-line block:

```rust
let desc_col = headers
    .iter()
    .position(|&h| h == "Description")
    .expect("Description header must exist");
```

The pattern is structurally identical — only the header-array binding name differs.

**Why it matters**: DUP-3 (mid-size structural duplication). The clone is small but creates two places that must be kept in lockstep if the column label ever changes (e.g., to `"Desc"` or to a localized string), and silently rots if one site adds a new label-collision check that the other lacks. A helper like `column_index(headers, "Description")` removes the magic-string repetition and centralizes the panic message.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a helper (e.g., fn column_index(headers: &[&str], name: &str) -> usize) and use it at both call sites
- [ ] #2 extension_cmd tests still pass; clippy clean under -D warnings
<!-- AC:END -->
