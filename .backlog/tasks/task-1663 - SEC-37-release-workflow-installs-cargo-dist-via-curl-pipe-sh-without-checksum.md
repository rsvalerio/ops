---
id: TASK-1663
title: >-
  SEC-37: release workflow installs cargo-dist via curl-pipe-sh without checksum
  verification
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
**File**: `.github/workflows/release.yml` → `Install dist` step

**What**: the release workflow installs `cargo-dist` by piping a downloaded
script to `sh`. The version is pinned (v0.31.0) and TLS is enforced, but there
is no checksum verification.

Separately, the `deps` job in `ci.yml` installed the latest `ops` GitHub release
via `jaxxstorm/action-install-gh-release` with no version pin — and then ran
`cargo deny check` directly, never using the installed binary.

**Why it matters**: low probability, but the consequence for the installer is a
supply-chain compromise of published release binaries. The `ops` install was
unpinned supply-chain surface for no benefit at all.

Found during the security review that also produced TASK-1660, 1661, 1662.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Risk is either mitigated (checksum, vendored installer) or explicitly accepted with a comment
- [x] #2 Vestigial `ops` install in `deps` job is removed or justified
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Shipped in PR #14 (squashed as `81e514d`), released in v0.36.1.

**AC #1 — accepted, with the reasoning recorded inline.** Checked before
deciding: **upstream publishes no checksum for this asset.** The v0.31.0 release
ships `.sha256` files for every binary tarball but none for
`cargo-dist-installer.sh`, so option 1 as written in the description is not
available.

Hardcoding a self-computed hash was rejected as security theatre: trust-on-first-use
against whatever is served today, and it would still not cover the tarball the
script goes on to download — verification of the first hop only, looking like
more than it is.

Kept: the URL pins an exact release (matching `cargo-dist-version` in
`dist-workspace.toml`) and `--proto '=https' --tlsv1.2` forbids protocol
downgrade. Residual exposure is a compromise of the axodotdev repo, which is the
same trust assumption as depending on cargo-dist at all.

**AC #2 — removed.** Confirmed vestigial first: the `deps` job's only remaining
step runs `cargo deny check` and never invokes `ops`. The misleading step name
`ops --raw deps` is now `cargo deny check`. Dropped the
`jaxxstorm/action-install-gh-release` dependency with it.

**Docs.** `docs/releasing.md` corrected — the claim that CI installs `ops` from
the latest release is gone, the Deps row reads `cargo deny check`, and the check
count went 6 → 7 for the new Workflow Guard job.
<!-- SECTION:NOTES:END -->
