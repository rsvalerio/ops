---
id: TASK-1427
title: >-
  SEC-21: about-card render_field emits attacker-controlled values to stdout
  without sanitise_line
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:22'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - SEC
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:209`

**What**: `render_field` formats `(key, value)` and writes directly via `format!("  {} {} {}", emoji, padded_key, styled(first))`. Values come from `ProjectIdentity` (Cargo.toml metadata: description, authors, repository, package.json fields) — all attacker-controlled when the workspace is hostile. No control-char sanitisation is applied; embedded ESC bytes reach stdout.

**Why it matters**: `ui::sanitise_line` (ui.rs:30) is the project's canonical defence and is applied to `ui::emit_to` and the dry-run preview path. The about-card is a parallel stdout writer with the same threat model (operator runs `ops about` against a hostile workspace).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Route field values through ui::sanitise_line (or an equivalent helper) before printing
- [ ] #2 Regression test: a project_identity field containing ESC / control bytes does not reach stdout verbatim
<!-- AC:END -->
