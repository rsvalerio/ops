---
id: TASK-0980
title: >-
  ERR-2: python format_authors emits bare email without angle brackets,
  diverging from node format_person
status: Triage
assignee: []
created_date: '2026-05-04 21:58'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:240-244`

**What**: When a pyproject author entry has only an email (`name = None, email = Some(e)`), `format_authors` produces `Some(e)` — the bare email string. The sister code in `extensions-node/about/src/package_json.rs::format_person` (line 171) wraps the same case as `Some(format!("<{e}>"))`. Both providers feed the same About card schema, so the same author shape renders inconsistently depending on whether the manifest is `package.json` or `pyproject.toml`.

**Why it matters**: Cross-stack rendering inconsistency in a user-visible field. The angle-bracketed form (`<a@example.com>`) is the conventional "name omitted" rendering used elsewhere in the codebase and matches the node sister. Without the brackets, a bare email author renders ambiguously next to "Name <email>" entries from the same authors list in a multi-author card.

**Candidate fix**:
```rust
(None, Some(e)) => Some(format!("<{e}>")),
```
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Python email-only author renders as <email>, matching node format_person
- [ ] #2 Test pins the rendering for an author with only email set
<!-- AC:END -->
