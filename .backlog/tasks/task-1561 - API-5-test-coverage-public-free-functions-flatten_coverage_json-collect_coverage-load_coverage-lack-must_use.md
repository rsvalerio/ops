---
id: TASK-1561
title: >-
  API-5: test-coverage public free functions flatten_coverage_json /
  collect_coverage / load_coverage lack #[must_use]
status: To Do
assignee:
  - TASK-1577
created_date: '2026-05-19 15:43'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:210,318,400`

**What**: Three crate-public functions return `Result<...>` carrying load-bearing data and lack `#[must_use]`:

- `pub fn flatten_coverage_json(raw: &serde_json::Value) -> Result<serde_json::Value, anyhow::Error>` (line 210)
- `pub fn collect_coverage(working_dir: &Path) -> Result<serde_json::Value, anyhow::Error>` (line 318)
- `pub fn load_coverage(data_dir: &Path, db: &DuckDb) -> Result<LoadResult, anyhow::Error>` (line 400)

`Result` is implicitly `must_use` via the std attribute on `Result` itself, so this is partially redundant — but the Ok variant carries semantic payload (the flattened JSON, the load report) that callers should explicitly consume. For `load_coverage` in particular, READ-5 (TASK-0808) was filed *specifically* because the prior signature returned `()` and silently dropped the LoadResult; pinning `#[must_use = "load report carries record_count health signal"]` would compile-time prevent that regression from recurring (e.g. via `let _ = load_coverage(...)`).

**Why it matters**: API-5 — discoverability + regression-proofing. Today nothing stops a caller from `let _ = load_coverage(dir, &db)?;` losing the zero-row warning signal that TASK-0808 explicitly added.

<!-- scan confidence: confirmed -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_coverage carries #[must_use = "..."] with a message referencing the record_count health signal
- [ ] #2 flatten_coverage_json and collect_coverage carry #[must_use] (or are documented as intermediate Result helpers exempt under workspace policy)
<!-- AC:END -->
