---
id: TASK-1513
title: >-
  PATTERN-1: resolve_root wraps a ? expression in Ok(...) instead of returning
  directly
status: Done
assignee:
  - TASK-1641
created_date: '2026-05-18 19:57'
updated_date: '2026-05-25 16:13'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:199-204`

**What**: `resolve_root` is written as:

```
fn resolve_root(&self, working_dir: &Path) -> Result<PathBuf, anyhow::Error> {
    if let Some(root) = &self.root {
        return Ok(root.clone());
    }
    Ok(find_workspace_root(working_dir)?)
}
```

The trailing `Ok(find_workspace_root(working_dir)?)` is the anti-pattern `Ok(x?)` — propagate the error with `?` only to immediately re-wrap the success in `Ok`. Since `FindWorkspaceRootError` implements `Into<anyhow::Error>` (via thiserror's std::error::Error blanket), the canonical form is `find_workspace_root(working_dir).map_err(Into::into)` or, more directly, `Ok(find_workspace_root(working_dir)?)` collapses to a `?` at a call site that returns `Result<PathBuf, anyhow::Error>`. Clippy's `needless_question_mark` lints this exact shape.

**Why it matters**: PATTERN-1 — keeps the codebase free of redundant `Ok(?)` shapes. Cosmetic but worth folding into the ERR-4 fix that already touches this function.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 resolve_root returns without the Ok(...?) wrapping
- [ ] #2 clippy --all-targets -- -W clippy::needless_question_mark is clean for the crate
<!-- AC:END -->
