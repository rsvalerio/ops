---
id: TASK-0709
title: 'PERF-1: wrap_text re-scans current_line O(n^2) via display_width per word'
status: Done
assignee:
  - TASK-0741
created_date: '2026-04-30 05:29'
updated_date: '2026-04-30 19:35'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:60`

**What**: Inside the per-word loop, `wrap_text` recomputes `display_width(&current_line)` on every iteration. `display_width` walks the entire string each call, so wrapping a description with N words performs O(N²) Unicode width work plus a `format!("{} {}", current_line, word)` allocation per accepted word.

**Why it matters**: Description wrapping runs on every render of `about` cards. While typical descriptions are short, the contract permits arbitrary input from manifest fields (Cargo.toml `description`, `package.json`). A pathological description (paste of release notes, hostile package metadata) turns card layout into a quadratic hot path. Tracking running width incrementally and pushing into a `String` instead of repeated `format!` reduces this to O(N).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Track current line width as an incrementing counter rather than rescanning current_line via display_width inside the loop
- [x] #2 Avoid format! reallocation per word; push the separator + word into the existing String
- [x] #3 Add a regression test or bench that demonstrates the O(N) bound on a long description (>10k chars)
<!-- AC:END -->
