---
id: TASK-2345
title: 'Drop engine: ops from the forge rust-ci call (PR #81): forge removed the input'
status: Done
assignee: []
created_date: '2026-09-29 19:01'
updated_date: '2026-10-02 20:25'
labels:
  - ci
dependencies: []
modified_files:
  - .github/workflows/ci.yml
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.github/workflows/ci.yml` on branch `ci/forge-rust-ci-and-lint-actions` (PR #81), lines ~34 and ~51

**What**: forge removed rust-ci's `engine` input (forge commit 2cb0dcf, `feat(rust-ci)!: run on ops only`, shipping on v1 as v0.7.0). rust-ci now always runs the ops jobs. A caller that still passes `engine: ops` fails with an unknown-input error, and PR #81 passes it. The rest of the call (`run-tests: false`, `run-msrv: true`) is unchanged.

**Why it matters**: once forge v0.7.0 moves v1, PR #81's CI breaks until the input is dropped.

**Origin**: forge rust-ci ops-only change, 2026-09-29.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops's rust-ci call passes no engine input, and its comments describe rust-ci as ops-only
- [x] #2 PR #81 (or its successor) is green against forge's ops-only rust-ci
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-09-29, forge #21 (28d5d96): rust-ci also dropped forge-ref, toolchain and use-sccache, and installs tools from the caller's mise.toml via jdx/mise-action. PR #81 must drop engine: ops and rely on ops's own mise.toml pinning ops >= 0.77.0 (enforced by the verify job), cargo-nextest, cargo-deny, cargo-machete, trivy, and rust with rustfmt,clippy.

Update (forge 1366e08): rust-ci's check names are now verify, test, deps, sec and msrv, with no ops- prefix. The ops >= 0.77.0 guard step was removed, so ops's own mise.toml must pin ops >= 0.77.0. If ops's ruleset requires checks produced by rust-ci, update them.

Done in #83 (merged 2026-10-02): the rust-ci call drops engine and calls forge rust-ci@v1 (v0.7.0), mise.toml pins ops 0.77.0 and the gate tools, qa is the pre-push gate, and the ruleset requires rust-ci / verify, rust-ci / qa and rust-ci / msrv.

<!-- SECTION:NOTES:END -->
