---
id: TASK-2302
title: 'Adopt forge publish-deb-dist so releases publish amd64/arm64 .debs to rsvalerio/apt'
status: Triage
assignee: []
created_date: '2026-09-26 19:32'
labels:
  - deb
  - apt
  - release
dependencies: []
modified_files:
  - .github/workflows/publish-deb.yml
  - .github/workflows/release.yml
  - dist-workspace.toml
  - README.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
ops currently ships Linux binaries through the GitHub Release tarballs, the `ops-installer.sh` curl installer (into `~/.cargo/bin`, with `install-updater = false`) and Linuxbrew. An apt package would give Debian and Ubuntu machines, including the arm64 Raspberry Pis, `apt install ops` and upgrades through `apt upgrade`, which none of the current Linux options offer.

Adopt forge's `publish-deb-dist.yml` (ops is its first consumer).

Changes:
- `.github/workflows/publish-deb.yml`: a local wrapper (same shape as `publish-homebrew.yml`) calling `rsvalerio/forge/.github/workflows/publish-deb-dist.yml@v1` with `description: "Batteries-included task runner for any stack"`, the maintainer, and the homepage. Pass `GH_APP_PRIVATE_KEY` explicitly, never `secrets: inherit`, because `workflow-guard` enforces this.
- `dist-workspace.toml`: set `publish-jobs = ["./publish-homebrew", "./publish-deb"]`.
- `.github/workflows/release.yml`: add `custom-publish-deb` (with SHA pins and an explicit `secrets:` block), and add it to `announce.needs` and its `if:` guard.
- Check that the my-cloud-ci GitHub App installation covers `rsvalerio/apt` for ops's credentials. my-cloud and oxydraw already push there.
- README "Installing": add an apt section (add the key and source, then `apt install ops`).

Rollout: run the first release with `dry-run: true`, then switch it off.

Blocked on: forge TASK-0006 (forge-testbed dry-run gate for publish-deb-dist must go green first). Forge prerequisites TASK-0003 (pool retention) and TASK-0005 (publish-deb-dist) are done. The rsvalerio/apt README publishers fix is tracked in the apt repo's backlog.

Moved from forge TASK-0007.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An ops release adds ops_<v>_amd64.deb and ops_<v>_arm64.deb to rsvalerio/apt pool in one commit
- [ ] #2 sudo apt install ops works on an amd64 host and an arm64 Raspberry Pi after adding the repo
- [ ] #3 ops workflow-guard still passes (SHA pins, explicit secrets)
- [ ] #4 ops README documents apt installation
<!-- AC:END -->
