---
id: TASK-0507
title: >-
  ERR-2: PomData parse_top_level <name> overwrite of artifact_id is
  order-dependent
status: Done
assignee:
  - TASK-0531
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 07:25'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:186`

**What**: parse_top_level unconditionally overwrites data.artifact_id when it sees a `<name>` element, regardless of prior `<artifactId>`. The artifactId branch above (~lines 175-179) is guarded by is_none(); the name branch is not. The two policies disagree.

**Why it matters**: Test parse_pom_basic confirms <name> wins over <artifactId>, but a second <name> would also win. Intent is undocumented and the asymmetric guards make the behaviour unclear; a POM with `<name>`, `<artifactId>`, then a stray duplicate `<name>` would silently flip values.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decide explicitly whether <name> or <artifactId> is the canonical display name and document it
- [ ] #2 Either guard the <name> branch with is_none() or guard both consistently and document last-write-wins
<!-- AC:END -->
