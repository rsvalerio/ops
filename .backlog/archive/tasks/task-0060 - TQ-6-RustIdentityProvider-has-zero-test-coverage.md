---
id: TASK-0060
title: 'TQ-6: RustIdentityProvider has zero test coverage'
status: Triage
assignee: []
created_date: '2026-04-14 20:54'
labels:
  - rust-test-quality
  - TestGap
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/about/src/identity.rs contains RustIdentityProvider with two substantial methods and zero tests. provide() (~90 lines) assembles a ProjectIdentity from Cargo.toml + DuckDB queries with multiple fallback chains (package -> workspace.package for version, description, edition, license, repository, authors). render_detail_section() has 5 match arms (coverage, dependencies, crates, stats, toolchain) each orchestrating query + format logic. The fallback chains in provide() are particularly risk-prone — incorrect ordering or missing fallbacks silently produce None fields. Related: CQ-6 notes this function's complexity.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 provide(): test field extraction from package-only manifest
- [ ] #2 provide(): test fallback to workspace.package fields when package fields are absent
- [ ] #3 provide(): test that missing optional fields produce None (not panic)
- [ ] #4 render_detail_section(): test at least one section returns expected output format
<!-- AC:END -->
