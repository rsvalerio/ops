---
id: TASK-1661
title: 'SEC-35: third-party GitHub Actions pinned to mutable tags, not commit SHAs'
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
**Files**: `.github/workflows/ci.yml`, `release.yml`, `publish-homebrew.yml`

**What**: every third-party action was pinned to a mutable version tag —
`actions/checkout@v6`, `taiki-e/install-action@v2`,
`mozilla-actions/sccache-action@v0.0.9`, `jaxxstorm/action-install-gh-release@v1`,
`actions/create-github-app-token@v3`, `actions/upload-artifact@v6`,
`actions/download-artifact@v7`. Tag references can be repointed by the upstream
maintainer, or by anyone who compromises their repo, with no visible change in
the consuming workflow.

**Why it matters**: the highest-risk reference is
`actions/create-github-app-token` in `publish-homebrew.yml` — it receives
`GH_APP_PRIVATE_KEY` and mints a token with write access to `homebrew-tap`. A
repointed tag there could exfiltrate the App key and push a malicious formula to
users. `actions/checkout` is next, running in every job of every workflow.

Found during the security review that also produced TASK-1660, 1662, 1663.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `actions/create-github-app-token` is pinned to a full commit SHA
- [x] #2 `actions/checkout` is pinned to a full commit SHA across all workflows
- [x] #3 Remaining third-party actions are pinned to full commit SHAs
- [x] #4 Each SHA pin has a trailing `# vN.N.N` comment for human readability
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shipped in PR #14 (squashed as `81e514d`), released in v0.36.1.

39 references pinned across all four workflows — 20 in `ci.yml`, 3 in
`publish-homebrew.yml`, 16 in `release.yml`. Each carries a trailing comment
with the exact semver the SHA resolved to, found by matching the SHA against the
repo's tag list rather than assuming the moving tag's name: checkout v6.1.0,
upload-artifact v6.0.0, download-artifact v7.0.0, create-github-app-token
v3.2.0, setup-rust-toolchain v1.17.0, sccache-action v0.0.9, install-action
v2.85.13, action-install-gh-release v1.14.0. Pins are to the SHA each tag
pointed at on 2026-08-15 — a pin, not an upgrade.

**New `workflow-guard` job in `ci.yml`** makes this durable: it greps every
workflow and fails if any `uses:` is not a 40-hex SHA. Verified in both
directions. Exemptions: local `uses: ./...` calls, and `rsvalerio/forge/...`.

**`rsvalerio/forge/...@v0.2.0` left version-pinned** — first-party, and
`bump.yml` documents intent to move back to `@v1`; a SHA pin would fight that.
It is also an exact release tag rather than a moving major. Worth revisiting:
this is the reference that receives `GH_APP_PRIVATE_KEY`, so pin it to a SHA if
it moves to `@v1`.

**Required by the dist interaction.** `dist plan` asserts `release.yml`
byte-matches its generated output, so the pins made it hard-fail. Resolved with
`allow-dirty = ["ci"]` in `dist-workspace.toml` — see the follow-up commit in
the same PR. Consequence: `release.yml` is now hand-maintained.

**Follow-up for the repo owner:** `workflow-guard` is not in the
`main-protection` ruleset, which names six checks explicitly (Test, Check, Lint,
Build, Deps, Format). Until added, the guard can fail without blocking merge.

Several actions are behind latest (checkout v6.1.0 vs v7.0.1, sccache-action
v0.0.9 vs v0.0.11, download-artifact v7.0.0 vs v8.0.1). Upgrading is out of
scope — this was about mutability, not currency — and is not filed.
<!-- SECTION:NOTES:END -->
