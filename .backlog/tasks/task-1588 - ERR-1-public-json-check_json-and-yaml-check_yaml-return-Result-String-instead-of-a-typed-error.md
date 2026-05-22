---
id: TASK-1588
title: >-
  ERR-1: public json::check_json and yaml::check_yaml return Result<(), String>
  instead of a typed error
status: Done
assignee:
  - TASK-1636
created_date: '2026-05-21 22:46'
updated_date: '2026-05-22 12:17'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/json.rs:5`, `extensions/config-checkers/src/yaml.rs:7`

**What**: Both module-level checker entrypoints expose `pub fn ... -> Result<(), String>`. The error variant flattens distinct failure modes (invalid UTF-8 vs parse error from `serde_json`/`json5`/`saphyr`) into a single opaque string before the caller can inspect it.

**Why it matters**: ERR-1 / ERR-10 — `Result<_, String>` in a library API forces callers to do substring matching to distinguish failures and prevents downstream code (or future callers outside `run_checker`) from rendering a richer message (line/column, source span). It also blocks anyhow's `.context()` chain from preserving structure if these were ever wired into a larger pipeline.

**Suggested fix**: Define a `CheckError` enum (or use `anyhow::Error`) with `InvalidUtf8(Utf8Error)` and `Parse(String)` variants (parser errors from the three backends can stay stringly-typed because their concrete error types diverge). Update `run_checker`'s `C` bound to match. The current consumer only needs `Display`, so the migration is local.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 json::check_json and yaml::check_yaml return a typed error (enum or anyhow::Error) rather than String
- [x] #2 run_checker and callers compile unchanged in behavior; tests still assert non-empty error display
<!-- AC:END -->
