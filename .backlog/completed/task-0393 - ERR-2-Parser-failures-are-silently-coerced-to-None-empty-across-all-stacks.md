---
id: TASK-0393
title: 'ERR-2: Parser failures are silently coerced to None/empty across all stacks'
status: Done
assignee:
  - TASK-0417
created_date: '2026-04-26 09:40'
updated_date: '2026-04-27 19:56'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:154` (also Node/Go/Maven providers)

**What**: parse_pyproject collapses both "file not found" and "TOML parse error" into the same None (.ok()? on read and on parse). Same pattern: parse_package_json (Node), read_mod_info (Go), read_workspace_members, read_package_metadata. Maven differs by surfacing computation_failed("could not parse pom.xml") but loses underlying error text.

**Why it matters**: A user with a malformed pyproject.toml/package.json sees the same fallback as one with no manifest at all — the failure is unobservable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Differentiate absent from malformed — return at minimum a tracing::debug!/warn! log including path and underlying error from parse failure branch (read errors with NotFound remain silent)
- [ ] #2 For Maven computation_failed, include the source error via .with_context(...) so callers can see why parsing failed; do not return a bare static string
<!-- AC:END -->
