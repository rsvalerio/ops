---
id: TASK-0498
title: >-
  READ-4: Python LicenseField::Table { file } surfaces the file path as the
  license string
status: To Do
assignee:
  - TASK-0532
created_date: '2026-04-28 06:10'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:192`

**What**: When pyproject.toml uses `license = { file = "LICENSE" }` (PEP 621), the parser returns text.or(file), so the About card displays `LICENSE` as the license name. This is a misleading value, not the SPDX identifier.

**Why it matters**: Users see a path string where they expect a license name. Either the file should be read+detected, or the field should be ignored for display purposes (or labeled differently). Quietly passing through the path is the worst of both.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decide and document: either skip the file variant (return None) or resolve it to a label like License file: LICENSE
- [ ] #2 Apply the chosen behavior in parse_pyproject
- [ ] #3 Test covers license = { file = LICENSE } and asserts the documented behavior
<!-- AC:END -->
