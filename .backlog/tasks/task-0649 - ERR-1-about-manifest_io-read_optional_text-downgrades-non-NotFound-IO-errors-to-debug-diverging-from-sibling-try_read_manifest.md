---
id: TASK-0649
title: >-
  ERR-1: about manifest_io::read_optional_text downgrades non-NotFound IO errors
  to debug, diverging from sibling try_read_manifest
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 04:53'
updated_date: '2026-04-30 17:46'
labels:
  - code-review-rust
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_io.rs:31` (debug!) vs `extensions/about/src/workspace.rs:88` (warn!)

**What**: `read_optional_text` was introduced as the consolidated replacement for the per-stack manifest readers (TASK-0622). It logs non-NotFound IO failures (EACCES, EIO, IsADirectory, etc.) at `tracing::debug!`. The sibling `try_read_manifest` in `extensions/about/src/workspace.rs:78-96` performs the same operation but logs at `tracing::warn!` — explicitly because TASK-0548 had previously been filed and resolved against the silent-skip variant.

The two coexisting helpers now have inconsistent severity policies for identical failure modes. Operators running with default tracing levels (info+) will see warnings for unreadable workspace-glob manifests but silence for unreadable about-page manifests, even though both surface the same user-visible symptom (a unit silently disappears from the about display).

**Why it matters**: Defeats the consolidation intent of TASK-0622 — the policy split is the exact divergence the unification was meant to prevent. A permission-denied or EIO on a manifest file is a real environment problem that the user needs to be told about (the unit's about card / project_units listing will be empty otherwise, with no diagnostic clue why).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either raise read_optional_text to tracing::warn! for non-NotFound errors (matching try_read_manifest), or document why about-page manifests warrant a quieter log severity than workspace-glob manifests
- [ ] #2 Add a regression test that captures the chosen severity (analogous to the workspace.rs::unreadable_manifest_is_skipped_not_silent test) so the policies cannot drift again
- [ ] #3 Migrate try_read_manifest to call read_optional_text once severities are unified, eliminating the duplicate skeleton (DUP-1)
<!-- AC:END -->
