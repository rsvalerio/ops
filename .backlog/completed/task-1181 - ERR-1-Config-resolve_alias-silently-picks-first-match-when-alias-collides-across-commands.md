---
id: TASK-1181
title: >-
  ERR-1: Config::resolve_alias silently picks first match when alias collides
  across commands
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 08:09'
updated_date: '2026-05-10 06:29'
labels:
  - code-review-rust
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:178`

**What**: `resolve_alias` linear-scans `commands` and returns the first command whose `aliases` list contains the input, with no detection of multiple commands declaring the same alias and no diagnostic. Two commands aliasing `lint` resolve invisibly to whichever appears first in the IndexMap.

**Why it matters**: Order-dependent alias resolution silently shadows the intended target across config layering / extension overlays; users debugging "wrong command ran" have no breadcrumb. The symmetric command/data-provider registration paths log every shadow but alias resolution does not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 validate_commands errors when two commands declare the same alias, OR resolve_alias emits tracing::warn! once with both candidate command names on first observed collision.
- [ ] #2 A test pins the chosen behavior: load a config with two commands declaring alias x, assert either a hard error from validation or a warn record naming both commands.
<!-- AC:END -->
