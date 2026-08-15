---
id: TASK-1659
title: Bump workflow omits --skip-ci so every bump commit retriggers CI and Bump
status: Done
assignee: []
created_date: '2026-08-08 22:32'
updated_date: '2026-08-15 00:00'
labels:
  - ci
  - release
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
File: `.github/workflows/bump.yml` (the `cog -vvv bump --auto` step)

**What**: `cog.toml` sets `skip_ci = "[skip ci]"`, and `docs/releasing.md:124` states "The bump commit uses `[skip ci]` to avoid retriggering CI." Neither is true in practice. In cocogitto 7.0.0 the `skip_ci` config key only *defines the string*; it is applied only when `cog bump` is invoked with the `--skip-ci` flag:

```
--skip-ci
    Add the skip-ci string defined in the cog.toml (or defaults to [skip ci]) to the bump commit
```

The workflow runs `cog -vvv bump --auto` without `--skip-ci`, so bump commits carry no marker. Verified empirically: neither `10395ea` nor `70dc7d0` (both `chore(version): v0.34.1`) contains `[skip ci]`.

**Why it matters**: Every bump commit pushed to `main` re-triggers the full CI workflow, which in turn re-triggers Bump via `workflow_run`. Normally the second Bump is a harmless no-op (a lone `chore` commit does not bump), so the cost is a wasted full CI + Bump cycle on every release.

It stops being harmless during recovery. On 2026-08-08, a manual `cog bump --auto` (run because the Aug 6 merge of PR #5 lost its push event to a GitHub Actions incident) pushed version commit `10395ea`. That push re-triggered CI -> Bump, and the server-side bump redid the same release: it produced a second `chore(version): v0.34.1` commit `70dc7d0`, tagged *that* one, and wrote a duplicate `v0.34.1` section into `CHANGELOG.md` that also listed the first version commit as a changelog entry. With `--skip-ci` in place the manual recovery would have completed without the duplicate.

**The obvious fix is unsafe — do not just add `--skip-ci`.** GitHub applies `[skip ci]` to the head commit message of *any* `push` event, and that includes **tag** pushes. `release.yml` triggers on `push: tags: v*` and its head commit is the bump commit itself. A bump commit carrying `[skip ci]` would therefore suppress the release run, and no binaries would ship. Trading a wasted CI cycle for silently skipped releases is a bad trade.

**Remaining scope**: if the duplicate-release cycle is still worth closing, it needs a mechanism that suppresses only CI and leaves the tag-triggered release alone — e.g. a condition in `ci.yml` that skips when the pushed head commit is a `chore(version):` bump, rather than a marker in the commit message.

**Partly overtaken by events** (2026-08-09): the docs claim is corrected, and the specific recovery failure described above is much less likely now — `cog.toml` no longer has `post_bump_hooks`, so a manual `cog bump --auto` commits and tags locally without pushing, and cannot re-trigger CI on its own.
**Fix**: add `--skip-ci` to the bump invocation in `bump.yml`, then correct or delete the claim at `docs/releasing.md:124` so the docs match behaviour.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A bump commit pushed to main does not trigger the CI workflow
- [x] #2 The tag push for that same bump still triggers release.yml and publishes binaries
- [x] #3 docs/releasing.md:124 matches actual behaviour
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
**Already resolved — closed on evidence, no code changes.** Fixed by the
`ci/dispatch-releases` work (`8468d67`, `617192d`), which removed the blocker
this task was parked on.

The task correctly refused the naive fix: adding `--skip-ci` was unsafe while
`release.yml` triggered on `push: tags: v*`, because GitHub applies `[skip ci]`
to the head commit of *any* push event, tag pushes included — so the marker
would have suppressed the release itself.

That constraint is gone. `release.yml` now triggers on `workflow_dispatch` and
`bump.yml` passes `release-workflow: release.yml` so forge dispatches the
release explicitly once the tag exists. Decoupling the release from the push
event is precisely the "mechanism that suppresses only CI and leaves the release
alone" the Remaining Scope section asked for — reached by making the release not
depend on a push event at all, rather than by special-casing `chore(version):`
in `ci.yml`.

**Verified in production on the v0.36.0 release, not from config alone:**

| SHA | Workflow | Event | Result |
|-----|----------|-------|--------|
| `1f28b44` (PR #13 merge) | CI | `push` | success 11:46 |
| `1f28b44` | Bump | `workflow_run` | success 11:48 |
| `02c0f81` (`chore(version): v0.36.0 [skip ci]`) | — | — | **no run — CI skipped** |
| — | Release | `workflow_dispatch` | success 11:51 → v0.36.0 published |

- AC #1: `02c0f81` is the first bump commit to carry `[skip ci]` (v0.35.0's
  `ade0e18` did not), and `gh run list` shows no workflow run against it. The
  CI → Bump cycle is closed. Reconfirmed on v0.36.1 (`6199d5a`).
- AC #2: satisfied in reframed form. The literal wording ("the tag push …
  triggers release.yml") no longer describes the design — releases are
  dispatched, not tag-triggered. The intent (binaries still publish) holds.
- AC #3: `docs/releasing.md` now reads "Bump commits carry `[skip ci]`, so they
  no longer re-trigger the CI + Bump cycle", with the flag-vs-config distinction
  spelled out.

The duplicate-release recovery failure that motivated this (the double
`v0.34.1`, `10395ea` + `70dc7d0`) is doubly guarded now: `cog.toml` has no
post-bump hooks, so a manual `cog bump --auto` commits and tags locally without
pushing.
<!-- SECTION:NOTES:END -->
