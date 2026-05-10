---
id: TASK-0705
title: >-
  READ-2: package.json RepositoryField::Object silently drops the 'directory'
  field used by monorepos
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:28'
updated_date: '2026-04-30 12:07'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:43-48,97-100`

**What**: `RepositoryField::Object` deserialises only `url`; the npm-supported `directory` field (used by monorepos to point at a sub-path within the repository, e.g. `{ "type": "git", "url": "...", "directory": "packages/foo" }`) is silently dropped. The About card therefore renders the same repository URL for every monorepo member, with no signal that the package lives at a sub-path. The pyproject reader does not have this concept (PEP 621 has no equivalent), so this is a Node-specific gap.

**Why it matters**: For users vendoring multi-package npm monorepos (e.g. babel, react-router, vue), the `repository` field on every member points at the root URL with `directory` distinguishing them; ops loses that distinction silently, so the About card looks like every package shares one Cargo-style root. Also a documentation drift: `RepositoryField::Object { url: Option<String> }` looks total but isn't.

<!-- scan confidence: candidates to inspect -->
- package_json.rs:43-48 (RepositoryField::Object schema)
- package_json.rs:97-100 (where the field is consumed)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 RepositoryField::Object preserves the  field (either appended to the normalised URL as a /tree/... fragment, or surfaced separately via ParsedManifest)
- [x] #2 test pins a monorepo-style package.json renders a repository value distinguishable from a sibling package in the same repo
- [x] #3 schema doc on RepositoryField::Object documents the directory handling so the field is no longer silently dropped
<!-- AC:END -->
