---
id: TASK-0047
title: 'TQ-3: DataRegistry::about_fields and detail_sections lack test coverage'
status: Triage
assignee: []
created_date: '2026-04-14 20:22'
labels:
  - rust-test-quality
  - TestGap
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/extension/src/data.rs — DataRegistry::about_fields() and DataRegistry::detail_sections() are public methods with no tests. The default DataProvider trait implementations for about_fields(), detail_sections(), and render_detail_section() are also untested. These methods are used by the about extensions to render dashboard and identity data. All other DataRegistry methods (register, get, provide, provider_names, schemas) are well-tested. Rules: TEST-5.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DataRegistry::about_fields() and detail_sections() each have at least one test
- [ ] #2 DataProvider default implementations for about_fields, detail_sections, render_detail_section are tested
<!-- AC:END -->
