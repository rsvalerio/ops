---
id: TASK-1948
title: >-
  ERR-1: expand_path silently discards the shellexpand error and falls back to
  the literal path
status: Done
assignee:
  - TASK-2002
created_date: '2026-08-27 15:48'
updated_date: '2026-08-28 21:30'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:353-358`

**What**:

    fn expand_path(path: &str) -> PathBuf {
        shellexpand::full(path).map_or_else(
            |_| PathBuf::from(path),
            |expanded| PathBuf::from(expanded.as_ref()),
        )
    }

The error arm throws the `LookupError` away and uses the unexpanded string. The same crate treats the same failure as fatal on the read side - `read_json_file` at `:248` does `shellexpand::full(path).with_context(|| format!("invalid path: {path}"))?`. So `--json-file '$UNSET/plan.json'` reports "invalid path", while `--out '$UNSET/plan.binary'` silently creates a directory literally named `$UNSET` and writes the plan artifact into it.

That divergence is more than cosmetic here: `expand_path` also feeds `cleanup_artifacts` (`:365`), so the swallowed error decides which file gets deleted.

**Why it matters**: ERR-1 - an error must be handled or propagated, not discarded. A silent fallback that changes where a secret-bearing artefact is written and later deleted is the wrong default, and it makes the two flag families behave inconsistently for identical input.

**Suggested fix**: make `expand_path` return `anyhow::Result<PathBuf>` with the same `invalid path: {path}` context as `read_json_file`, and propagate from `run_terraform_pipeline`; `cleanup_artifacts` can log and skip on failure rather than deleting a wrong path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 expand_path returns a Result and its callers propagate or explicitly log the expansion failure
- [x] #2 --out and --json-file report the same invalid path error for the same unexpandable input
- [x] #3 A test asserts an unexpandable --out value produces an error rather than a directory named after the literal variable reference
<!-- AC:END -->
