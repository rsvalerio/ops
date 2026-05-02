---
id: TASK-0867
title: >-
  ARCH-1: extensions-python manifest_cache is unbounded process-global with no
  eviction
status: Triage
assignee: []
created_date: '2026-05-02 09:21'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/manifest_cache.rs:21,49-53`

**What**: static CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<Arc<toml::Value>>>>> with no eviction. The doc comment says "bounded leak: at most one entry per project root probed", but in long-running hosts (LSP server, daemonized ops mode, cargo test reuse) the entry count grows monotonically.

**Why it matters**: Acceptable for one-shot CLI; pre-existing footgun for any future host that calls into this provider repeatedly. Each entry retains a fully-parsed toml::Value (kilobytes).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either bound the cache (LRU with small N) or scope it via a constructor parameter passed through Context rather than a static
- [ ] #2 Document the maximum residency in the module comment (safe under N<=K probed roots; do not link from a long-running host)
- [ ] #3 Add a debug-build assertion that the cache size stays under a sensible threshold (e.g., 1024 entries)
<!-- AC:END -->
