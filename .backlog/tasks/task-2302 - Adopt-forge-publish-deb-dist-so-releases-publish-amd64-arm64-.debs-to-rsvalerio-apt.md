---
id: TASK-2302
title: 'Adopt forge publish-deb-dist so releases publish amd64/arm64 .debs to rsvalerio/apt'
status: In Progress
assignee: []
created_date: '2026-09-26 19:32'
updated_date: '2026-09-27 09:25'
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
- [x] #3 ops workflow-guard still passes (SHA pins, explicit secrets)
- [x] #4 ops README documents apt installation

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Wired with dry-run: true ahead of forge TASK-0006 (user decision 2026-09-27): .github/workflows/publish-deb.yml wrapper -> forge publish-deb-dist@v1, publish-jobs += ./publish-deb, release.yml custom-publish-deb (explicit secrets, contents: read added since forge declares it) gated into announce, README apt section. workflow-guard pins/inherit checks pass locally; actionlint clean. Remaining: confirm the my-cloud-ci App installation covers rsvalerio/apt, cut a release and inspect the deb-ops-<v> artifact, then drop dry-run and verify AC #1/#2.

PR #71 review: README apt section removed until dry-run is off (nothing is in the pool yet) — re-add it with the dry-run flip. custom-publish-deb now grants only contents: read (the apt push uses forge's minted App token); wrapper has a top-level contents: read. Forge stays @v1 per the workflow-guard exemption (first-party, version-pinned by decision, as bump.yml).

Dry-run verified on release v0.68.0 (run 36277531411): custom-publish-deb green, announce green. Artifact deb-ops-0.68.0 holds ops_0.68.0_amd64.deb and ops_0.68.0_arm64.deb (Package ops, Section utils, /usr/bin/ops + /usr/share/doc/ops/{LICENSE,README.md}); the amd64 binary runs (ops 0.68.0), the arm64 one is aarch64 ELF. The my-cloud-ci App minted a token for rsvalerio/apt and checked it out, so the installation covers it. pool-update.sh staged both .debs in one diff. Next: drop dry-run, re-add the README apt section, then verify AC #1/#2 on the following release.

Dry-run dropped and README apt section re-added in PR #72 (merged 2026-09-27). #72 held only ci/docs/chore commits, so Bump produced no release; the next feat/fix release is the first live publish. Remaining: AC #1 (check the rsvalerio/apt pool commit from that release) and AC #2 (apt install on amd64 and the Pi).

Pre-check for AC #2 (2026-09-27): the v0.68.0 dry-run .debs install with apt-get install ./file.deb on debian:bookworm-slim, amd64 native and arm64 under qemu, and ops --version prints 0.68.0. AC #2 itself (install from the apt repo on a real amd64 host and the Pi) waits for the first live publish. Forge follow-up: rsvalerio/forge PR #14 fixes the consuming.md permissions example and records this evidence on forge TASK-0006.

<!-- SECTION:NOTES:END -->
