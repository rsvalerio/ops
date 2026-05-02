---
id: TASK-0896
title: >-
  READ-7: tools list wildcard arm prints Debug representation of unknown
  ToolStatus variants to users
status: Triage
assignee: []
created_date: '2026-05-02 09:47'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/cli/src/tools_cmd.rs:47

**What**: TASK-0759 replaced the silent _ => dim('?') arm with other => (dim(format!({other:?})), Cow::Owned(format!( ({other:?}) ))). While this fixes the silent collapse, it now ships the Debug representation to end users for any future variant. Debug output is not a stable user contract — it changes if a variant gains fields, gets renamed, or is restructured. The catch-all also defeats the compiler exhaustiveness check.

**Why it matters**: A future ToolStatus::Pending { since: Instant } would surface as (Pending { since: Instant { ... } }) in user-facing output. The fix swapped one bad UX for another and the compiler still wont warn when a new variant lands.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove the other => arm and match each variant explicitly so the compiler errors when a variant is added
- [ ] #2 If a stable user-facing string is desired, implement Display on ToolStatus so the text is intentional contract, then call format!({status}) in the CLI
- [ ] #3 Add a comment on ToolStatus requiring CLI updates when variants are added
<!-- AC:END -->
