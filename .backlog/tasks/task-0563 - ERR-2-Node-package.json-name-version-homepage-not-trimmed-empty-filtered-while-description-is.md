---
id: TASK-0563
title: >-
  ERR-2: Node package.json name/version/homepage not trimmed/empty-filtered
  while description is
status: Triage
assignee: []
created_date: '2026-04-29 05:03'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:96-107`

**What**: parse_package_json trims and empty-filters description but leaves raw.name, raw.version, and raw.homepage un-trimmed (homepage is empty-filtered but not trimmed). A package.json with name as empty or whitespace-only string lands whitespace strings in ProjectIdentity instead of triggering fallback.

**Why it matters**: Inconsistent normalization yields blank-looking About cards rather than the dir-name fallback used for missing manifests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 name/version trimmed-and-empty-filtered identically to description
- [ ] #2 homepage trimmed before the empty-filter
- [ ] #3 Same normalization applied in extensions-python/about/src/lib.rs at project.name / project.version
<!-- AC:END -->
