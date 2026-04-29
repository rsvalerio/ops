---
id: TASK-0613
title: 'PATTERN-1: parse_action_line splitn(4) cap silently drops trailing annotations'
status: Done
assignee:
  - TASK-0641
created_date: '2026-04-29 05:20'
updated_date: '2026-04-29 12:06'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:208`

**What**: splitn(4, " ") requires exactly `name from -> to`. If cargo ever annotates the line (e.g. `Updating serde v1 -> v2 (yanked)`), the splitn(4) cap means a trailing annotation is silently dropped onto the to token. The starts_with_known_verb path catches it via warn, but the to-token corruption goes unnoticed.

**Why it matters**: A future "Updating X v1 -> v2 (yanked)" emits with to="v2" and annotation lost. Format-drift class same as TASK-0472.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either parse with regex/iterator that tolerates annotation, or warn when leftover tokens after to
<!-- AC:END -->
