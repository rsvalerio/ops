---
id: TASK-0691
title: >-
  READ-2: maven pom <licenses> single-line shortcut undocumented vs <scm>
  rejection symmetry
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:16'
updated_date: '2026-04-30 11:14'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:253-271`

**What**: The single-line `<scm>...</scm>` and `<licenses>...</licenses>` detector uses `line.matches("<scm>").count() == 1` to reject duplicate openers, but the corresponding `<licenses>` check counts only `<licenses>` occurrences, not `<license>` (singular). A pathological line containing two `<license>` children matches the single-line shortcut and silently keeps only the first license — documented behaviour, but the asymmetry with `<scm>` (where duplicate openers are explicitly rejected) is undocumented.

**Why it matters**: Future readers can't tell whether the asymmetry is intentional. The "deterministic" guarantee in parse_pom_duplicate_scm_opener_deterministic is only pinned for <scm>.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add a comment beside <licenses> clarifying that single-line multi-license shape is allowed and 'first license wins' (matching the multi-line handle_licenses policy), or rejected like <scm>
- [x] #2 Optional: a test pinning the chosen behaviour
<!-- AC:END -->
