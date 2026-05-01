---
id: TASK-0806
title: >-
  DUP-3: read_crate_metadata duplicates manifest parsing that the cargo-toml
  extension already exposes
status: Triage
assignee: []
created_date: '2026-05-01 06:02'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:86-130` vs `extensions-rust/cargo-toml/src/lib.rs:191-203`

**What**: read_crate_metadata does fs::read_to_string + toml::from_str + manual get(package).and_then walks for name/version/description. The ops_cargo_toml::CargoTomlProvider implements this exact parse and exposes typed CargoToml::package_name / package_version. The Rust units provider depends on ops_cargo_toml already.

**Why it matters**: Two parsers in one extension means two places to fix when the schema evolves (TASK-0707 already had to apply a fix to one but not the other). TASK-0620 documented the same DUP-3 antipattern across stacks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace read_crate_metadata body with a call into ops_cargo_toml::CargoToml::parse(content) and pull out package_name, package_version, plus a description accessor (adding one if Package does not expose it)
- [ ] #2 Tracing posture preserved (NotFound silent, other read errors at debug, parse errors at warn)
- [ ] #3 Tests in units::tests continue to pass without modification
<!-- AC:END -->
