---
id: TASK-0019
title: "canonical_id has CC ~8 and duplicates resolve logic"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, FN-6, CL-5, medium, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/mod.rs:176-197`
**Anchor**: `fn canonical_id`
**Impact**: This function has cyclomatic complexity ~8 (3 `contains_key` checks + alias check + 2 loops with `.any()` + fallthrough). It reimplements the same three-source priority lookup as `resolve()` (direct → config alias → stack aliases → extension aliases) but for a different return type (`&str` vs `&CommandSpec`). The fallthrough to `id` at line 196 silently returns the original unresolved ID, which may pass validation downstream. If a new command source is added, `resolve`, `resolve_alias`, and `canonical_id` all need updating in lockstep — a latent consistency hazard.

**Notes**:
Unify by having `canonical_id` delegate to a shared lookup that returns `Option<(&str, &CommandSpec)>`, then extract the key. Alternatively, make `resolve` return the canonical name alongside the spec. This eliminates the duplicated priority logic and reduces `canonical_id` to a one-liner.
