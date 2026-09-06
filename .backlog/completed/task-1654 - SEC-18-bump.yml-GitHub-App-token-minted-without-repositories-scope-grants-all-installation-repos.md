---
id: TASK-1654
title: >-
  SEC-18: bump.yml GitHub App token minted without repositories: scope grants
  all installation repos
status: Done
assignee: []
created_date: '2026-06-07 11:05'
updated_date: '2026-06-07 11:32'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.github/workflows/bump.yml:18-27`

**What**: The `actions/create-github-app-token@v3` step sets `owner: rsvalerio` but omits `repositories:`. Per the action's semantics, owner-without-repositories scopes the minted token to **every repository in the owner's installation** — not just the repo being bumped. The sibling mint in `release.yml` (publish-homebrew) correctly restricts with `repositories: homebrew-tap`.

**Why it matters**: Least privilege (OWASP A01). The bump job only needs to push commits/tags to its own repository; a compromised step in this job (or a poisoned dependency of cargo-edit/cocogitto installed in a later step) could use the over-scoped token against any other repo the `my-cloud-ci` app is installed on, including homebrew-tap. The token is short-lived, but scoping it costs one line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 bump.yml token-mint step includes a repositories: entry restricting the token to the repository being bumped
- [x] #2 Bump workflow still pushes the version commit/tag and triggers the downstream release workflow
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added repositories: ${{ github.event.repository.name }} to the bump.yml app-token mint, scoping the token to the bumped repo only (mirrors release.yml's repositories: homebrew-tap). Push/tag still uses the same app token so the downstream release workflow still triggers.
<!-- SECTION:NOTES:END -->
