---
id: TASK-0661
title: >-
  CL-5: CommandRegistry and DataRegistry diverge on duplicate-registration
  policy
status: Done
assignee:
  - TASK-0740
created_date: '2026-04-30 05:12'
updated_date: '2026-04-30 19:13'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:144-155` and `crates/extension/src/extension.rs:104-109`

**What**: `DataRegistry::register` is first-write-wins (warn + return) while `CommandRegistry::insert` is last-write-wins (overwrite + record). Both registries are used for "extension contributions" with opposite resolution policies; neither site cross-references the other.

**Why it matters**: A reader cannot tell from either site that the policies diverge; both are documented in their own comments without cross-reference. CL-5 consistent-patterns guidance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either align both policies (likely first-wins everywhere — security-trusted built-ins should not be shadowed) or document the split with a cross-reference comment on each insert/register site so the asymmetry is intentional and discoverable
<!-- AC:END -->
