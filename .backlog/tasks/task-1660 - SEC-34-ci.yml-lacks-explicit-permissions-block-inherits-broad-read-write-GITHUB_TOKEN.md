---
id: TASK-1660
title: >-
  SEC-34: ci.yml lacks explicit permissions block — inherits broad read/write
  GITHUB_TOKEN
status: Done
assignee: []
created_date: '2026-08-14 00:00'
updated_date: '2026-08-15 00:00'
labels:
  - security
  - ci
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.github/workflows/ci.yml`

**What**: `ci.yml` had no top-level `permissions:` block. Unless the repo's
default token permissions are restricted in Settings → Actions → General, the
`GITHUB_TOKEN` for every PR-triggered run gets read/write access to contents,
packages, pull-requests and more. None of the jobs need write access.

By contrast `bump.yml` already set `permissions: contents: read`.

**Why it matters**: a compromised third-party action step running under this
workflow could use the write-capable token to push code, merge PRs or modify
releases. On a workflow that runs against every PR, that is the widest-reach
token in the repo.

Found during the security review that also produced TASK-1661..1663.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `ci.yml` has an explicit top-level `permissions:` block with `contents: read`
- [x] #2 All CI jobs still pass after the change
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shipped in PR #14 (squashed as `81e514d`), released in v0.36.1. See that
commit's `ci(security):` section for the full rationale.

Confirmed `contents: read` was sufficient before applying it: `ci.yml`
references no `secrets.*` and no `GITHUB_TOKEN` directly, and every job only
reads. Declared at the top level so it caps all jobs and a future job must opt
into more explicitly. All seven CI jobs pass on the merged commit.

`publish-homebrew.yml` still has no `permissions:` block — it is a
`workflow_call` and inherits from its caller. Covered in TASK-1662.
<!-- SECTION:NOTES:END -->
