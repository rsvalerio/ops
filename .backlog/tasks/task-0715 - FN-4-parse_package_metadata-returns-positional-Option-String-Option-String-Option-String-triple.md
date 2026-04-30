---
id: TASK-0715
title: >-
  FN-4: parse_package_metadata returns positional (Option<String>,
  Option<String>, Option<String>) triple
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:30'
updated_date: '2026-04-30 12:45'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:137`

**What**: `parse_package_metadata` returns `(Option<String>, Option<String>, Option<String>)` representing `(name, version, description)`. The closure callers pass also returns the same anonymous triple. Argument-order is the only thing keeping name from being read as version at every call site.

**Why it matters**: FN-4 / API-2: prefer named fields (struct or newtype) over positional tuples once the tuple has more than two same-typed components. A future rename or reordering inside `parse` is a silent semantics flip — there is no compiler help, and `cargo expand` of the closures all look identical. A `PackageMetadata { name, version, description }` struct (or even reusing `ParsedManifest` for the three fields) would make the contract self-describing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace the (Option<String>, Option<String>, Option<String>) return type with a named struct (e.g. PackageMetadata { name, version, description })
- [x] #2 Update both callers (Node package.json, Python pyproject.toml) and the unit tests
- [x] #3 Optionally, route through identity::ParsedManifest if the fields fit, to keep one canonical name/version/description shape
<!-- AC:END -->
