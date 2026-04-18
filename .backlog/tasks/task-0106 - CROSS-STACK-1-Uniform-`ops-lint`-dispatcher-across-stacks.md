---
id: TASK-0106
title: 'CROSS-STACK-1: Uniform `ops lint` dispatcher across stacks'
status: Triage
assignee: []
created_date: '2026-04-18 10:12'
labels:
  - cross-stack
  - enhancement
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: Make `ops lint` work on every stack regardless of whether the stack's idiomatic linter command is named `clippy`, `vet`, `lint`, or other.

**Why it matters**: Users switching projects (Rust → Go → Python) expect `ops lint` to Just Work. Today Rust uses `clippy`, Go uses `vet`, and TF has no default `lint` at all. The 7-command baseline for `.default.<stack>.ops.toml` keeps idiomatic names, so a dispatcher layer is the cleanest way to close the gap.

**Approaches to consider**:
1. Core-level alias table (e.g. `lint → clippy` for Rust, `lint → vet` for Go) resolved before command lookup.
2. Synthesize a hidden `lint` group in `default_commands()` when no real `lint` key exists, referencing the per-stack linter.
3. Add real `lint` keys in each default TOML that alias the idiomatic command (duplication).

**Context**: Baseline contract landed in `.default.*.ops.toml` with a parity matrix in README. See `/Users/rsvaleri/.claude/plans/yes-lets-plam-and-wild-tiger.md`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Running `ops lint` on a Rust project executes clippy
- [ ] #2 Running `ops lint` on a Go project executes go vet
- [ ] #3 Running `ops lint` on a Python project executes ruff check
- [ ] #4 No duplicate `lint` definitions in default TOML templates
- [ ] #5 Existing idiomatic names (clippy, vet, format) remain callable directly
<!-- AC:END -->
