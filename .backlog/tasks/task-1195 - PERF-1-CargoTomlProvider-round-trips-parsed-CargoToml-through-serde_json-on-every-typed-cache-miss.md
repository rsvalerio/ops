---
id: TASK-1195
title: >-
  PERF-1: CargoTomlProvider round-trips parsed CargoToml through serde_json on
  every typed-cache miss
status: To Do
assignee:
  - TASK-1262
created_date: '2026-05-08 08:14'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:218-237`; `extensions-rust/about/src/query.rs:237-255`

**What**: The cargo_toml provider parses the manifest with toml::from_str into a typed CargoToml, then calls serde_json::to_value(&manifest) and returns a serde_json::Value. Downstream, load_workspace_manifest reads that JSON via ctx.cached(...), deep-clones it ((**cached).clone()), and serde_json::from_value parses it back into a typed CargoToml. The typed-cache miss path pays: TOML→typed → typed→Value → Value clone (deep) → Value→typed.

**Why it matters**: The whole point of the typed-manifest cache (TASK-0558) is to avoid this round-trip; carrying the typed value through Context (e.g. as an Arc<CargoToml>, or by exposing CargoTomlProvider::provide_typed) removes the JSON bridge entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_workspace_manifest cache-miss path calls serde_json::from_value zero times (or removes the JSON intermediate altogether).
- [ ] #2 CargoTomlProvider exposes a typed accessor for in-process consumers; the existing serde_json::Value path stays for cross-extension consumers.
<!-- AC:END -->
