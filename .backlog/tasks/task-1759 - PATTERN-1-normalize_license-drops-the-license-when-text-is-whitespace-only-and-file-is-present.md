---
id: TASK-1759
title: >-
  PATTERN-1: normalize_license drops the license when text is whitespace-only
  and file is present
status: Triage
assignee: []
created_date: '2026-08-27 11:19'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:234-243`

**What**: The match arms are ordered so that a `Table` variant with *any* `text` value — including a whitespace-only one — claims the match before the `file` arm can be considered:

```rust
LicenseField::Table { text: Some(t), .. } => trim_nonempty(Some(t)),
LicenseField::Table { file: Some(f), .. } => { ... }
```

For `license = { text = "  ", file = "LICENSE" }`, arm 2 matches, `trim_nonempty` returns `None`, and the function returns `None`. The `file` fallback at arm 3 is unreachable for that input, so the About card shows no license at all even though the manifest declares one.

**Why it matters**: this defeats the two policies the function was written to implement, in combination. ERR-2 / TASK-0704 added trim-and-drop so a blank field would not render as an empty bullet; the `License file:` arm exists so a `file`-only declaration is still surfaced. A manifest that has both — a blank `text` and a real `file` — hits the intersection and loses information that the code already has in hand. It is exactly the shape produced by a template or generator that emits every PEP 621 key and leaves unfilled ones as empty strings, so it is not a contrived input.

The fix is to try the arms in value order rather than in field order: take the first of `text` (trimmed, non-empty) then `file` (trimmed, non-empty), rather than matching on which fields are `Some`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 normalize_license falls through to the file form when text is present but trims to empty
- [ ] #2 license = { text = "  ", file = "LICENSE" } yields Some("License file: LICENSE")
- [ ] #3 license = { text = "  ", file = "  " } still yields None
- [ ] #4 Existing behaviour for text-only, file-only and empty-table inputs is unchanged and covered by tests
<!-- AC:END -->
