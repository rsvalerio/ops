---
id: TASK-0597
title: >-
  ERR-7: parse_deny_output substitutes literal "unknown" for missing package
  names
status: Done
assignee:
  - TASK-0639
created_date: '2026-04-29 05:19'
updated_date: '2026-04-29 11:00'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:370`

**What**: When neither advisory.package nor graphs[0].krate.name is present, the package field defaults to "unknown". format_advisories/format_deny_section then renders this as a real package, indistinguishable from a crate literally named "unknown". No tracing::debug records the substitution.

**Why it matters**: ERR-7. Operators can`t tell whether cargo-deny emitted a package-less diagnostic (likely schema drift) or the workspace genuinely depends on "unknown".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either model the missing case as Option<String> in entry struct (preferred) or emit tracing::debug when substitution fires
- [x] #2 Format helpers render unknown case visibly distinct (e.g. <no package>)
<!-- AC:END -->
