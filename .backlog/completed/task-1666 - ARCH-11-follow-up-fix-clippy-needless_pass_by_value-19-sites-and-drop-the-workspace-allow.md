---
id: TASK-1666
title: >-
  ARCH-11 follow-up: fix clippy::needless_pass_by_value (19 sites) and drop the
  workspace allow
status: Done
assignee: []
created_date: '2026-08-15 20:15'
updated_date: '2026-08-16 10:23'
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
- [x] #1 Every needless_pass_by_value site is fixed or carries a site-local #[allow] with a reason
- [x] #2 The needless_pass_by_value = "allow" line is removed from [workspace.lints.clippy]
- [x] #3 cargo clippy --all-targets --workspace -- -D warnings passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Done. All 19 sites resolved; the `needless_pass_by_value` allow is out of `[workspace.lints.clippy]`.

**Correction to this task's own premise.** It claimed several sites were
constrained by the `'static` bounds on the runner's tokio spawns. That was
wrong — I wrote it from the signature without reading the body.
`spawn_parallel_tasks` never consumes its `Arc` parameters; it `Arc::clone`s
them per task inside the loop, so the `'static` bound applies to the clones,
not the params. Taking `&Arc<T>` works, and the call site in `run_plan_parallel`
was doing `self.cwd.clone()` / `self.vars.clone()` / `Arc::clone(&self.workspace_cache)`
purely to satisfy the by-value signature — three refcount bumps per parallel
plan, now gone.

Resolution by kind:

**Derived `Copy` (7 sites, zero call-site churn).** The parameter was a
lightweight borrow-bundle or a fieldless enum that is matched, never consumed;
`needless_pass_by_value` does not fire for `Copy` types:
- `ListRow<'a>`, `BorderArgs<'a>`, `CommandSelector<'a>` — every field already `Copy` (`&str`, `usize`, `u16`, `&[String]`)
- `ThemeAction`, `AboutAction`, `RunBeforeCommitAction`, `RunBeforePushAction` — clap subcommand enums, all unit variants

**Changed to a reference (11 sites).** `spawn_parallel_tasks` (3 Arcs),
`on_step_output`, `right_pad_with_border`, `run_init`/`run_init_to`,
`run_plan_pipeline`/`run_plan_pipeline_to`/`run_plan_pipeline_to_with_tty`,
`run_config_checker`, `with_path`, and two test helpers
(`render_with`, `test_display_with_config`). Call sites updated across
`main.rs`, `subcommands.rs`, `init_cmd.rs` and the runner/theme/plan tests.
The reference conversions surfaced four `needless_borrow` warnings where the
body then double-borrowed the new reference; those were fixed too.

**Site-local allow (1 site).** `expand_err_to_io` in
`crates/runner/src/command/build.rs` — used point-free as
`.map_err(expand_err_to_io)` at four call sites, so a `&ExpandError` parameter
would force a closure at each. Consuming the error also matches
`From<E> for io::Error` semantics. Reason recorded at the site per
`docs/clippy.md`.

Gates: `ops verify` 7/7, `ops qa` 3/3.
<!-- SECTION:NOTES:END -->
