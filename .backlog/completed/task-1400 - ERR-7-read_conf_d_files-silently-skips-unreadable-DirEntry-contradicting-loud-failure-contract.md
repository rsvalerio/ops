---
id: TASK-1400
title: >-
  ERR-7: read_conf_d_files silently skips unreadable DirEntry, contradicting
  loud-failure contract
status: Done
assignee:
  - TASK-1453
created_date: '2026-05-13 18:09'
updated_date: '2026-05-13 20:40'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:248-262`

**What**: A `DirEntry` whose `entry?` access fails is logged at warn and skipped; the rest of the overlay merge proceeds. The sibling parse-error branch (around line 269) documents "loud failures" as the contract.

**Why it matters**: Asymmetric error handling: unreadable entries silently disappear while malformed TOML aborts the load. A CI overlay drop (permission flip, racing rename) can go missing without aborting, producing a config that differs from what the operator authored.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DirEntry iteration errors abort the overlay load like parse errors do, or the contract is updated and documented
- [ ] #2 Test covers a forced DirEntry error
<!-- AC:END -->
