---
id: TASK-0034
title: >-
  CQ-6: RustIdentityProvider::provide exceeds FN-1 with repetitive field
  extraction
status: Done
assignee: []
created_date: '2026-04-14 19:41'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/about/src/identity.rs:228 — provide() is 91 lines (FN-1 threshold: 50). Six fields use the identical pattern: pkg.and_then(|p| p.FIELD.as_str()).or(ws_pkg.and_then(|wp| wp.FIELD.as_deref())).map(|s| s.to_string()). Rules: FN-1, READ-6 (consistent patterns). Refactoring: extract a helper like fn resolve_field<T>(pkg: Option<&Package>, ws_pkg: Option<&WorkspacePackage>, getter: impl Fn(&_) -> Option<&str>) -> Option<String> to reduce repetition and bring provide() under the 50-line threshold.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 provide() ≤50 lines
- [ ] #2 Repetitive field extraction replaced by shared helper
<!-- AC:END -->
