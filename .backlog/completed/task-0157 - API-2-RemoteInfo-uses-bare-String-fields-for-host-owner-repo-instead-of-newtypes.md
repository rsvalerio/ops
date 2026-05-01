---
id: TASK-0157
title: >-
  API-2: RemoteInfo uses bare String fields for host/owner/repo instead of
  newtypes
status: Done
assignee: []
created_date: '2026-04-22 21:23'
updated_date: '2026-04-23 07:42'
labels:
  - rust-code-review
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/remote.rs:5-12`

**What**: `RemoteInfo { host: String, owner: String, repo: String, url: String }` uses bare `String` for four semantically distinct fields. At call sites — e.g. `provider.rs:38-41` that clones each into `Option<String>` — there is nothing preventing a caller from swapping `host` with `repo` or passing the wrong field to URL templating. `url` additionally has an implicit invariant ("normalized https URL, no credentials, no .git suffix") that the type does not encode.

**Why it matters**: Newtype wrappers (`Host(String)`, `Owner(String)`, `RepoName(String)`, `RepoUrl(String)`) would prevent argument-order bugs and give a place to hang the normalization invariant for `url`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Introduce newtype wrappers for host/owner/repo/url or document the decision to keep them bare
<!-- AC:END -->
