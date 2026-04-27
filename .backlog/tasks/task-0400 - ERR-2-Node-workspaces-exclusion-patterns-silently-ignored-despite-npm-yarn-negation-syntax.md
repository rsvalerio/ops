---
id: TASK-0400
title: >-
  ERR-2: Node workspaces exclusion patterns silently ignored despite npm/yarn
  negation syntax
status: Done
assignee:
  - TASK-0417
created_date: '2026-04-26 09:48'
updated_date: '2026-04-27 19:56'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:139-167` (`resolve_member_globs`)

**What**: Yarn/npm `workspaces` arrays support exclusion entries like `"!packages/internal-*"`. The resolver detects the leading `!` (line 144) and `continue`s on that entry, but never filters the directories matched by previous `packages/*` patterns. The accompanying test (`exclusion_pattern_ignored`, lines 280-298) acknowledges this with the comment "still shows both" — confirming the behavior is known but not implemented.

**Why it matters**: Users writing yarn workspace exclusions get no warning and no effect — the excluded subpackage still appears in `project_units` output. This silently diverges from yarn/npm semantics and from the sibling `extensions-python/about/src/units.rs` (lines 92, 98-104) which correctly implements `[tool.uv.workspace].exclude`. Per ERR-2 (silently coerced) and PATTERN consistency, either implement the filter or fail loudly when an `!` entry is seen.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 resolve_member_globs in extensions-node/about/src/units.rs filters resolved entries against -prefixed patterns (matching the Python provider semantics), OR the test/docstring is updated and a tracing warning is emitted when an exclusion entry is encountered
- [ ] #2 Test exclusion_pattern_ignored is renamed/updated to assert that the excluded entry is in fact removed from collect_units output
<!-- AC:END -->
