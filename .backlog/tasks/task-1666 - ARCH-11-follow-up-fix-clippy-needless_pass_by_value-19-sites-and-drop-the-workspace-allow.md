---
id: TASK-1666
title: >-
  ARCH-11 follow-up: fix clippy::needless_pass_by_value (19 sites) and drop the
  workspace allow
status: Triage
assignee: []
created_date: '2026-08-15 20:15'
labels:
  - rust-code-review
  - arch
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-0137 enabled clippy::pedantic workspace-wide. `needless_pass_by_value` is allowed in [workspace.lints.clippy] because the 19 remaining sites need signature *and* call-site changes, several constrained by the 'static bounds on tokio spawn in crates/runner/src/command/parallel.rs. Sites: crates/cli/src/{hook_shared.rs:101,init_cmd.rs:18,row.rs:38,subcommands.rs:34,68,271,288,344}, crates/core/src/text.rs:216, crates/runner/src/{command/build.rs:268,command/parallel.rs:264-267,display.rs:536}, crates/theme/src/configurable.rs:616,640, extensions-terraform/plan/src/lib.rs:104. Note run_before_commit/run_before_push take Arc<Config> by value and main.rs Arc::clone()s at the call site — passing &Arc removes a refcount bump per invocation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every needless_pass_by_value site is fixed or carries a site-local #[allow] with a reason
- [ ] #2 The needless_pass_by_value = "allow" line is removed from [workspace.lints.clippy]
- [ ] #3 cargo clippy --all-targets --workspace -- -D warnings passes
<!-- AC:END -->
