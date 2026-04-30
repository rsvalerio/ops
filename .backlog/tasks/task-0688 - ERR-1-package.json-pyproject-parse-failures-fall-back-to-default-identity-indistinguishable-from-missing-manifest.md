---
id: TASK-0688
title: >-
  ERR-1: package.json/pyproject parse failures fall back to default identity
  indistinguishable from missing manifest
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:16'
updated_date: '2026-04-30 10:13'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:68-74` and `extensions-python/about/src/lib.rs:172-178`

**What**: When the manifest fails to parse, both providers return `None`, which causes `parse_package_json(...).unwrap_or_default()` / `parse_pyproject(...).unwrap_or_default()` at the call sites to silently fall back to a fully-default identity — the tracing::warn! is the only signal anything went wrong.

**Why it matters**: A malformed `package.json` or `pyproject.toml` should arguably surface as a `DataProviderError` (or at least a populated identity with `name` from dir + a "manifest failed to parse" diagnostic), not the same shape as "no manifest". Currently a typo is indistinguishable in card output from a folder with no manifest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Decide and document the policy (silent fallback vs. error vs. annotated identity); if silent fallback stays, add a tracing field (recovery = "default-identity") so log scraping can correlate 'warn parse error' → 'default identity emitted'
<!-- AC:END -->
