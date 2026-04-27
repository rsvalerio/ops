---
id: TASK-0407
title: >-
  SEC-21: ExecCommandSpec::display_cmd renders args with space-only join, no
  shell escaping
status: To Do
assignee:
  - TASK-0419
created_date: '2026-04-26 09:52'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:390-396` (`display_cmd`); also `expanded_args_display` at line 399-411 with the same pattern

**What**: `display_cmd` formats a command for the step line by concatenating `program` and the `args` slice with a single space: `format!("{} {}", self.program, self.args.join(" "))`. There is no shell-quoting: an arg that contains a space, an embedded quote, a `;`, a newline, or backtick is rendered indistinguishably from multiple separate args or a literal shell metachar.

Example: spec `{program: "cargo", args: ["build", "--config", "evil = "; rm -rf /""]}` renders as `cargo build --config evil = "; rm -rf /"` in the step line / dry-run output / TAP file. A user reading the line sees what looks like a multi-step shell pipeline; the real exec is a single argv list with one tokenised argument.

**Why it matters**: This is purely a display-fidelity / SEC-21-class information issue, not an injection (the actual exec uses argv directly via `tokio::process::Command::args`, no shell). But the dry-run path is *the* place users audit `.ops.toml` before running it, and the existing copy in `cli/src/run_cmd/dry_run.rs` plus the runner step lines both go through `display_cmd`. A misleading rendering can cause an operator to greenlight a config they would have rejected if the args were quoted with `shlex::try_join` or equivalent.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 display_cmd / expanded_args_display either shell-quote args containing whitespace/metacharacters (e.g. via shlex::try_join) or produce an unambiguous representation (e.g. each arg in single quotes)
- [ ] #2 Step-line rendering, dry-run output, and TAP capture all consume the new representation
- [ ] #3 Regression test asserts an arg containing a space and a quote round-trips through display_cmd in a way the user can disambiguate from two separate args
<!-- AC:END -->
