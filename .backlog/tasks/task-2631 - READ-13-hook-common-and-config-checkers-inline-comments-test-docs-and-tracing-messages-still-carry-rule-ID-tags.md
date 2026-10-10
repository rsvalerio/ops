---
id: TASK-2631
title: 'READ-13: hook-common and config-checkers inline comments, test docs, and tracing messages still carry rule-ID tags'
status: Triage
assignee: []
created_date: '2026-10-10 22:19'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/hook-common/src/config.rs
  - extensions/hook-common/src/git.rs
  - extensions/hook-common/src/git_state.rs
  - extensions/hook-common/src/install.rs
  - extensions/config-checkers/src/json.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/config.rs:47,81,90,104,187`, `extensions/hook-common/src/git.rs:135,146,175,211,213,216,332,550,745,769` plus the tracing strings naming an "SEC-14 anchor" in `canonical_anchor`, `extensions/hook-common/src/git_state.rs:54,93`, `extensions/hook-common/src/install.rs:109,231,330,338,376,397,408,572`, `extensions/config-checkers/src/json.rs:236`

**What**: The wave-70 doc cleanups (TASK-2569/2570/2571/2572/2573/2559) stripped the rule-ID prefixes from `///` doc comments per their acceptance criteria, but the same crates still carry the identical artifact in places those tasks scoped out: inline `//` body comments (`// ERR-7: ...`, `// SEC-25: ...`, `// PATTERN-1: ...`, `// ERR-5 / SEC-32: ...`), test-doc mentions (`SEC-33 regression:`, `SEC-33 open_refusing_symlinks guard`), and operator-facing tracing messages in `canonical_anchor` that say "SEC-14 anchor" — a rulebook ID a reader of the log cannot resolve.

**Why it matters**: READ-13 — self-reports of which review guideline a change followed are process artifacts: they go stale while looking authoritative, and in tracing output they reach operators who have no access to the rulebook. The technical prose around each tag is durable and must stay; only the tag goes.

**Origin**: discovered during TASK-2624 (wave 70) while closing the member doc-stripping tasks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->
