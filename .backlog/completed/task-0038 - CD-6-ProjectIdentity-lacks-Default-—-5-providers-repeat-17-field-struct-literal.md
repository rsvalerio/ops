---
id: TASK-0038
title: >-
  CD-6: ProjectIdentity lacks Default — 5 providers repeat 17-field struct
  literal
status: Done
assignee: []
created_date: '2026-04-14 20:11'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-2
  - DUP-7
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Anchor**: ProjectIdentity { ... }
**Crate(s)**: extensions/about, extensions-go/about, extensions-java/about, extensions-rust/about, crates/core
**Rule**: DUP-2 (3+ similar functions), DUP-7 (Default + struct update syntax)

ProjectIdentity has 17 fields. Five providers construct it with full struct literals, each setting ~10 fields to None/empty/default:
- extensions-go/about/src/lib.rs:117 (Go provider)
- extensions-java/about/src/lib.rs:103 (Maven provider)
- extensions-java/about/src/lib.rs:351 (Gradle provider)
- extensions-rust/about/src/identity.rs:297 (Rust provider)
- extensions/about/src/lib.rs:258 (fallback provider)

Most fields default to None/vec![]/empty string. Each provider only sets 3-6 stack-specific fields, yet must spell out all 17.

**Fix**: Add a constructor like `ProjectIdentity::new(name, stack_label)` that defaults all optional fields, then use struct update syntax (`..ProjectIdentity::new(name, stack_label)`) in each provider. Alternatively, derive/impl Default and use a builder. This would reduce each init site from ~17 lines to ~5-8 lines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ProjectIdentity has a Default impl or constructor that eliminates repeated None/empty field assignments
- [ ] #2 Each provider uses struct update syntax or builder, listing only stack-specific fields
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
CD audit re-confirmation: ProjectIdentity construction verified in 5 locations: Go provider (extensions-go/about/src/lib.rs:117), Maven provider (extensions-java/about/src/lib.rs:103), Gradle provider (extensions-java/about/src/lib.rs:351), Rust provider (extensions-rust/about/src/identity.rs), and generic fallback (extensions/about/src/lib.rs:258). All set loc, file_count, dependency_count, coverage_percent to None and languages to vec![]. A Default impl or ProjectIdentity::minimal(name, stack_label, project_path) builder would eliminate 5x repeated 17-field struct literals.
<!-- SECTION:NOTES:END -->
