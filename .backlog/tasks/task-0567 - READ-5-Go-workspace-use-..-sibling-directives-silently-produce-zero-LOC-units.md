---
id: TASK-0567
title: 'READ-5: Go workspace use ../sibling directives silently produce zero-LOC units'
status: Done
assignee:
  - TASK-0641
created_date: '2026-04-29 05:03'
updated_date: '2026-04-29 12:04'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:30-46, 67-74`

**What**: normalize_module_path only strips a leading ./ and trailing /. A go.work with use ../shared (which Go cmd/go accepts) lands ../shared in ProjectUnit.path, which is matched against tokei_files.file via starts_with — tokei paths never begin with parent-dir, so the unit always reports zero files / zero LOC with no diagnostic.

**Why it matters**: Out-of-tree workspace members silently render as empty cards, masking a real configuration shape and a real coverage gap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Parent-dir use directives are either resolved relative to cwd (canonical path) before being placed in path, or surfaced via tracing::warn as out-of-tree members
- [ ] #2 Test covers use ../shared not silently zeroing tokei stats
<!-- AC:END -->
