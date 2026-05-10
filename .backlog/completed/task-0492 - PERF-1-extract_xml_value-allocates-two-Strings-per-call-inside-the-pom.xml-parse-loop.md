---
id: TASK-0492
title: >-
  PERF-1: extract_xml_value allocates two Strings per call inside the pom.xml
  parse loop
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 06:09'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:195-207`

**What**: extract_xml_value calls format!("<{tag}>") and format!("</{tag}>") on every invocation. parse_top_level calls it 5 times per line; for a typical multi-hundred-line pom.xml that yields ~10 small allocations per line.

**Why it matters**: Pure overhead in the parse hot path. The tags are static &str known at the call site; the open/close strings could be precomputed once or matched via byte search without intermediate allocations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_xml_value (or its callers) avoids per-line allocation for open/close markers
- [ ] #2 parse_top_level constructs the open/close pairs once or uses a const-friendly scheme
- [ ] #3 Existing pom tests still pass
<!-- AC:END -->
