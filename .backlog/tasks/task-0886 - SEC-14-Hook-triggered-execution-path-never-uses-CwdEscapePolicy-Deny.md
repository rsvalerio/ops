---
id: TASK-0886
title: 'SEC-14: Hook-triggered execution path never uses CwdEscapePolicy::Deny'
status: Done
assignee: []
created_date: '2026-05-02 09:37'
updated_date: '2026-05-02 12:09'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/runner/src/command/build.rs:74-100 and crates/cli/src/hook_shared.rs

**What**: CwdEscapePolicy::Deny is defined but never constructed outside tests (the variant carries #[allow(dead_code)] which confirms it). The hook-triggered execution path (the binary that runs on every git commit/push) goes through build_command which hardcodes WarnAndAllow, so a .ops.toml landed by a coworker PR can specify cwd = "/etc" or cwd = "../../" and the runner only logs a warning before spawning. The doc on CwdEscapePolicy::Deny openly states the gap: "Kept in the public API so hook-triggered entry points can opt in once they thread a policy through CommandRunner. Currently only constructed in tests; the default interactive path stays WarnAndAllow to avoid a behaviour change for existing users." The whole rationale for SEC-14 being a separate policy was that hooks fire automatically without the maintainer reviewing the manifest, so the fail-closed branch must actually fire.

**Why it matters**: This is the documented threat model in the doc-comment itself. A teammate who pushes a malicious .ops.toml can have the next git commit execute a command rooted at any absolute path on the filesystem, with only a tracing warning. SEC-23 / TASK-0500 already closed the absolute-path bypass under Deny, but Deny is never used. Remediation: CommandRunner should take an explicit policy at construction, and the hook entry crates (run-before-commit, run-before-push) should opt in to Deny.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Hook-triggered exec paths construct CommandRunner with CwdEscapePolicy::Deny
- [ ] #2 Interactive ops <cmd> invocations continue to use WarnAndAllow (no behaviour change)
- [ ] #3 Regression test installs a hook, plants a .ops.toml whose cwd escapes the workspace, runs the hook entry point, and asserts the spawn is refused
- [ ] #4 #[allow(dead_code)] is removed from the Deny variant since it is now reachable
<!-- AC:END -->
