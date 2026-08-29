---
id: TASK-2028
title: >-
  PERF-1: load_workspace_manifest now runs a canonicalizing ancestor walk on
  every call, including cache hits
status: Triage
assignee: []
created_date: '2026-08-28 20:27'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - extensions-rust/about/src/manifest.rs
  - extensions-rust/about/src/manifest_cache.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/manifest.rs` (`load_workspace_manifest` → `resolve_workspace_root`), consumed by `extensions-rust/about/src/manifest_cache.rs` (`probe`)

**What**: CL-3 / TASK-1762 made the typed-manifest cache keyed by the resolved workspace root so two cwds inside one workspace share an entry. That requires resolving the root *before* the cache probe, so `find_workspace_root_strict` now runs on every `load_workspace_manifest` call — cache hits included.

`find_workspace_root_strict` is not free: it walks up to `MAX_ANCESTOR_DEPTH` ancestors and, per the SEC-25 / TASK-1204 hardening, `fs::canonicalize`s each candidate's parent. Before the change the cache-hit path cost one `HashMap` probe plus one `stat` of `<cwd>/Cargo.toml`; it now additionally pays that walk. Four providers hit the cache per `ops about` run, so the walk runs four times per invocation against the same cwd.

This is a deliberate trade — the previous hot path was cheap and wrong (it resolved members against the cwd) — but the cost is real and the module doc that carefully justifies the mtime+len freshness design as the hot-path budget no longer describes what the hot path actually does.

**Why it matters**: PERF-1 asks that repeated work in a hot loop be amortised. The root for a given cwd cannot change within a process, so this is a pure memoization opportunity: a small `cwd -> resolved root` map (or folding the resolution into the existing cache as a second index) restores the one-stat hit path while keeping the root-keyed entry. It is a bounded, self-contained change in one crate.

**Origin**: discovered during TASK-1993 (code-review-plan-wave159) while fixing TASK-1762 — the wave's own side effect, filed rather than fixed because adding a second cache is beyond the CL-3 fix's scope and wants its own eviction and invalidation reasoning.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The workspace-root resolution for a given cwd is performed at most once per process, so a typed-manifest cache hit costs no additional canonicalizing ancestor walk
- [ ] #2 The memoization states its key, value, maximum size and invalidation expectation, matching the contract style already used by manifest_cache
- [ ] #3 The cache remains keyed by the resolved workspace root so two cwds inside one workspace still share a single entry (manifest_cache::tests::cache_is_keyed_by_workspace_root_not_cwd still passes)
<!-- AC:END -->
