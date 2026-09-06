---
id: TASK-1662
title: 'SEC-36: publish-homebrew inherits all secrets via secrets: inherit'
status: Done
assignee: []
created_date: '2026-08-14 00:00'
updated_date: '2026-08-15 00:00'
labels:
  - security
  - ci
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.github/workflows/release.yml` → `custom-publish-homebrew` job

**What**: the job called `.github/workflows/publish-homebrew.yml` with
`secrets: inherit`, which forwards every repo secret to the called workflow. It
needs exactly one, `GH_APP_PRIVATE_KEY`. `bump.yml` already demonstrated the
minimal-secrets pattern.

**Why it matters**: if `publish-homebrew.yml` is modified, or an action in it is
compromised, the blast radius is every repo secret rather than the one it needs.
Low severity because the called workflow is in-repo, but least privilege is
cheap here.

Found during the security review that also produced TASK-1660, 1661, 1663.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `custom-publish-homebrew` passes only the secrets the called workflow needs, or a comment documents why `secrets: inherit` is required
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shipped in PR #14 (squashed as `81e514d`), released in v0.36.1.

Scoped rather than documented-as-required — the `dist generate` caveat turned
out to be solvable, so the stronger option was available.

`publish-homebrew.yml` now declares `GH_APP_PRIVATE_KEY` as a required secret on
its `workflow_call` trigger, and `release.yml` passes exactly that. Confirmed by
reading the workflow that it is the only secret needed: `GH_APP_CLIENT_ID` is a
**variable** (`vars.*` resolves in called workflows without being passed), and
the artifact actions use the automatic `github.token`.

**Durability.** `dist` emits `secrets: inherit`, so a second step in the new
`workflow-guard` job fails the build if it reappears. Verified in both
directions. `dist-workspace.toml` was checked for a native way to express this;
there is none, hence the guard plus `allow-dirty = ["ci"]` (see TASK-1661).

**Left alone deliberately:** the escalated `permissions: {id-token: write,
packages: write}` dist grants `custom-publish-homebrew`. Neither looks necessary
— the tap checkout uses the App token, not `GITHUB_TOKEN` — but this is the live
release path, tightening it cannot be exercised without cutting a release, and a
wrong guess breaks publishing. `publish-homebrew.yml` is user-owned and survives
regeneration, so a job-level `permissions:` block there is the place to narrow
it if desired.
<!-- SECTION:NOTES:END -->
