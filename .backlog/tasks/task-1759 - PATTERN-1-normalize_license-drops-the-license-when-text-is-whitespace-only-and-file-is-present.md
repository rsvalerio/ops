---
id: TASK-1759
title: >-
  PATTERN-1: normalize_license drops the license when text is whitespace-only
  and file is present
status: Done
assignee:
  - TASK-1992
created_date: '2026-08-27 11:19'
updated_date: '2026-08-28 20:04'
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
- [x] #1 normalize_license falls through to the file form when text is present but trims to empty
- [x] #2 license = { text = "  ", file = "LICENSE" } yields Some("License file: LICENSE")
- [x] #3 license = { text = "  ", file = "  " } still yields None
- [x] #4 Existing behaviour for text-only, file-only and empty-table inputs is unchanged and covered by tests
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`normalize_license` now matches the `Table` variant once and resolves in value
order — `trim_nonempty(text).or_else(|| trim_nonempty(file).map(...))` — so a
whitespace-only `text` no longer claims the match and strands the `file` arm.
The three previously separate `Table` arms collapse into one, which also
removes the unreachable catch-all.

Tests: `blank_license_text_falls_through_to_the_file_form`,
`blank_license_text_and_file_still_drops`, and
`license_table_text_only_and_empty_table_are_unchanged`; the pre-existing
`license_file_form_is_labeled` and
`whitespace_only_license_and_author_components_are_dropped` cover the
file-only and text-only-blank cases unchanged.
<!-- SECTION:NOTES:END -->
