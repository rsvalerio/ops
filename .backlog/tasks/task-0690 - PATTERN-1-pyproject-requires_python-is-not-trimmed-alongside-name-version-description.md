---
id: TASK-0690
title: >-
  PATTERN-1: pyproject requires_python is not trimmed alongside
  name/version/description
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:16'
updated_date: '2026-04-30 10:31'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:193`

**What**: `requires_python` is passed through verbatim while `name`, `version`, and `description` get `trim_nonempty` (per the comment at lines 187-189). A whitespace-only `requires-python = "  "` would render as `Python   ` in stack_detail.

**Why it matters**: Cross-field consistency — the same fix that landed for description/name/version (TASK-0563) was missed here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 out.requires_python = trim_nonempty(p.requires_python);
- [x] #2 Test pins that whitespace-only requires-python produces stack_detail = None (or 'uv' if uv is present), not 'Python   '
<!-- AC:END -->
