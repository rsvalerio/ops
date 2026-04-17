---
id: TASK-0071
title: 'DUP-2: canonical_id/resolve_alias/list_command_ids repeat 3-source iteration'
status: To Do
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:227`

**What**: canonical_id (L202), resolve_alias (L240), and list_command_ids (L258) each chain/loop over the same trio (config.commands, stack_commands, extension_commands) with subtly different access patterns.

**Why it matters**: The three command sources are a conceptual unit but leak through every resolver; refactoring requires changing three parallel implementations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a command_sources() iterator or small struct aggregating the three maps
- [ ] #2 Rewrite canonical_id/resolve_alias/list_command_ids in terms of the shared abstraction
<!-- AC:END -->
