---
id: TASK-2250
title: >-
  DUP-2: ops-core table/ui escape range sets still define their own
  control-codepoint lists instead of the shared is_unsafe_display_char predicate
status: Triage
assignee: []
created_date: '2026-09-08 16:02'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - crates/core/src/table.rs
  - crates/core/src/ui.rs
priority: low
ordinal: 156000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/table.rs:155-165`, `crates/core/src/ui.rs:88-91`

**What**: TASK-2116 promoted the strictest control/bidi/zero-width rejection
policy to `ops_core::text::is_unsafe_display_char` and unified the
*drop-based* consumers (ops-git, ops-about). Two *escape-based* consumers
remain with their own independent range sets: `ops-core::table` and
`ops-core::ui` escape a different (unnamed) set of codepoints rather than
deriving the "what needs escaping" decision from the shared predicate. The
drop-vs-escape decision correctly stays at each call site; the *set* is the
duplicated part.

**Why it matters**: The original DUP-2 finding named four implementations;
two were unified in wave2 (TASK-2236) and these two were left because their
decision differs (escape, not reject). A codepoint added to the shared
predicate still will not reach the table/ui escape sets, so rendered table
cells and UI text can keep carrying a codepoint every drop-based surface
now rejects — the same silent drift the shared predicate was created to
prevent.

**Origin**: discovered during TASK-2236 (wave2) while fixing TASK-2116.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 table.rs and ui.rs derive the codepoint set they escape from ops_core::text::is_unsafe_display_char (or a named superset documented against it), keeping the escape-vs-drop decision local
<!-- AC:END -->
