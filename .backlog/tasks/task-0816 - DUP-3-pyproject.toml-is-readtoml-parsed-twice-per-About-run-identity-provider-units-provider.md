---
id: TASK-0816
title: >-
  DUP-3: pyproject.toml is read+toml-parsed twice per About run (identity
  provider + units provider)
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:03'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:170-173` vs `extensions-python/about/src/units.rs:53-62`

**What**: parse_pyproject and read_workspace_members each call manifest_io::read_optional_text and toml::from_str on the same pyproject.toml. Each provider runs once per About invocation; both fire on the same path during a single render.

**Why it matters**: A non-trivial pyproject (real-world examples are 2--10 KB and toml::from_str is allocation-heavy) is parsed twice. TASK-0620 covered the metadata helper duplication; this is the root manifest re-read.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache the parsed RawPyproject (e.g. via a shared lazy in Context extras, or a single-shot helper in ops_about)
- [ ] #2 Both providers consume the same parsed value
<!-- AC:END -->
