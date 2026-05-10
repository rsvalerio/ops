---
id: TASK-0520
title: 'READ-1: Workspace.exclude declares serde alias matching its own field name'
status: Done
assignee:
  - TASK-0533
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 18:00'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:250`

**What**: Workspace.exclude is declared with `#[serde(default, alias = "exclude")] pub exclude: Vec<String>`. The alias is identical to the field name, so it has no effect.

**Why it matters**: Misleading attribute — readers infer the alias maps a different on-disk key (kebab vs snake) but it does not. Cargo only spells this `exclude`. default-members above has the same shape but with a real kebab alias.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Remove alias = "exclude" from Workspace.exclude
- [x] #2 Existing parse tests for [workspace].exclude still pass
<!-- AC:END -->
