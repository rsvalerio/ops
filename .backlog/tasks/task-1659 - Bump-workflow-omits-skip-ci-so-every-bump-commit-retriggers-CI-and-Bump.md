---
id: TASK-1659
title: Bump workflow omits --skip-ci so every bump commit retriggers CI and Bump
status: To Do
assignee: []
created_date: '2026-08-08 22:32'
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

**Fix**: add `--skip-ci` to the bump invocation in `bump.yml`, then correct or delete the claim at `docs/releasing.md:124` so the docs match behaviour.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 bump.yml invokes cog bump with --skip-ci so bump commits carry the [skip ci] marker
- [ ] #2 A bump commit pushed to main does not trigger the CI workflow
- [ ] #3 docs/releasing.md:124 matches actual behaviour
<!-- AC:END -->
