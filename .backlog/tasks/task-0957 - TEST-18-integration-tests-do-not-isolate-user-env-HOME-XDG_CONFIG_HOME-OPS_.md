---
id: TASK-0957
title: >-
  TEST-18: integration tests do not isolate user env
  (HOME/XDG_CONFIG_HOME/OPS_*)
status: Triage
assignee: []
created_date: '2026-05-04 21:46'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `tests/integration.rs:48-50, 76-114, 304-356`

**What**: The `ops()` helper returns `Command::cargo_bin("ops")` without sanitising `HOME`, `XDG_CONFIG_HOME`, or `OPS_*` env vars. Several tests (`cli_init_*`) run `ops` against an empty tempdir and assume no parent config influences behaviour; on a developer machine with `~/.ops.toml` or `~/.config/ops/...`, results diverge from CI.

**Why it matters**: TEST-18 (isolated state per test). Reproducibility across dev/CI machines depends on the developer not having local ops config — a brittle precondition.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops() helper clears or pins HOME/XDG_CONFIG_HOME to a known empty tempdir
- [ ] #2 ops() clears OPS_* env vars before each invocation
- [ ] #3 Tests that depend on user-config absence document the env precondition they rely on
<!-- AC:END -->
