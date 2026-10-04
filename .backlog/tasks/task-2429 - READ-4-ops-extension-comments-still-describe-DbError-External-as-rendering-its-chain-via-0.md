---
id: TASK-2429
title: 'READ-4: ops-extension comments still describe DbError::External as rendering its chain via {0:#}'
status: Triage
assignee: []
created_date: '2026-10-04 15:28'
labels:
  - code-review-rust
  - READ
dependencies: []
modified_files:
  - crates/extension/src/error.rs
  - crates/extension/tests/public_api.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/error.rs:~80` (`SharedError` Display), `crates/extension/tests/public_api.rs:~416` (`computation_failed_plain_display_flattens_chain`)

**What**: Both comments cite `DbError::External` as an example of a link whose Display embeds its own sources via `{0:#}` ("parity with `DbError::External`", "duplication is cosmetic"). Since TASK-2406 `DbError::External` displays only its own layer and exposes the anyhow error as source, so the example is wrong and the duplication it excuses no longer occurs for DbError.

**Why it matters**: The comment justifies a design choice with a fact that is no longer true; a reader will look for a duplicated chain that does not exist.

**Origin**: discovered during TASK-2416 while fixing TASK-2406.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The comments no longer cite DbError::External as a Display that embeds its sources
<!-- AC:END -->
