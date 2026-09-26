---
id: TASK-2276
title: 'ops sec: close default coverage gaps (repo-root files, .trivyignore.yaml, dev deps)'
status: Done
assignee: []
created_date: '2026-09-25 20:53'
updated_date: '2026-09-26 12:35'
labels:
  - feature
  - sec
dependencies: []
modified_files:
  - crates/cli/src/sec_cmd.rs
  - README.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while replacing a hand-written `make trivy` target with `ops sec` in oxydraw (a monorepo: `backend/` Rust + `frontend/` bun, gated by `ops verify qa` run inside each). The Makefile target is `trivy fs --scanners vuln,secret,misconfig --include-dev-deps --ignorefile .trivyignore.yaml .` from the repo root; `ops sec` with defaults covers less, in three ways:

1. **Scan root is the invoking directory only.** `run_sec` scans `root` (cwd). When `ops sec` runs inside a subproject (as `qa` does in `backend/`), files at the repo root or in sibling non-stack dirs are never scanned: in oxydraw the root `Dockerfile`, `packaging/docker/Dockerfile.build`, and anything under `.github/`. `ops --dry-run sec` in `backend/` shows `[skip] misconfiguration (no Dockerfile, Kubernetes, or IaC files found)` even though the repo ships two Dockerfiles. Running `ops sec` from the repo root works but nothing in the per-project `qa` does that.

2. **`.trivyignore.yaml` is never honoured.** `trivy_argv` passes no `--ignorefile`, so Trivy falls back to its default `.trivyignore` (plain format, cwd-relative). A project using the YAML format (needed for path-scoped rules with `statement:`), or keeping the ignore file at the repo root while scanning a subdir, gets its suppressions silently dropped and the scan fails on already-triaged findings.

3. **Dev dependencies are excluded.** `Scan::Vuln` does not pass `--include-dev-deps`, so npm/yarn/gradle devDependencies (build tooling, test runners) are not checked. Note Trivy 0.74 only supports that flag for npm, yarn, gradle (not bun/pnpm), so the gain is stack-dependent.

Design questions: for (1), scan the git toplevel vs. add non-stack paths vs. make `sec` a repo-level command that `qa` does not duplicate per subproject. Two subprojects each scanning the toplevel would double the work and output. For (3), default-on vs. opt-in (`--include-dev-deps` / `.ops.toml` key); default-on matches "security gate" intent but may surface findings in tooling that never ships.

Out of scope, possible follow-up: a license scan (`--scanners license`), which is informational/noisy and not a gate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Running `ops sec` inside a subproject of a git repo also covers IaC/misconfig markers outside that subproject (e.g. a root `Dockerfile`), or the dry-run plan explicitly says they are out of scope and how to include them
- [x] #2 A monorepo whose `qa` runs `sec` in several subprojects does not scan the same files twice
- [x] #3 `ops sec` discovers `.trivyignore.yaml` and `.trivyignore` at the scan root and the git toplevel and passes it via `--ignorefile`; the dry-run plan names the ignore file used (or says none)
- [x] #4 Vulnerability scans include dev dependencies by default for stacks Trivy supports, with a documented way to opt out (or the reverse, with the decision recorded in the README)
- [x] #5 Tests cover subproject-vs-toplevel root selection, ignore-file discovery (both formats, both locations), and the dev-deps argv
- [x] #6 README `ops sec` section documents the new defaults

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decisions (confirmed with user): (1) stay scoped to the invoking dir; the plan and a one-line note on live runs name IaC files elsewhere in the git toplevel (excluding the scan-root subtree, skip dirs, and sibling dirs with their own .ops.toml) and suggest the new `ops sec --repo` flag, which scans the git toplevel. That keeps per-subproject qa from scanning shared files twice. (2) --include-dev-deps is on by default for the vuln scan; `--no-dev-deps` opts out. Ignore file: first of .trivyignore.yaml/.trivyignore at the scan root, then the toplevel, passed as --ignorefile to every scan (fs and config). Toplevel is found by walking up to a .git entry (dir or file), with no git subprocess. Verified end to end against Trivy 0.74: a root .trivyignore.yaml suppresses a github-pat finding when scanning from a subproject.
<!-- SECTION:NOTES:END -->
