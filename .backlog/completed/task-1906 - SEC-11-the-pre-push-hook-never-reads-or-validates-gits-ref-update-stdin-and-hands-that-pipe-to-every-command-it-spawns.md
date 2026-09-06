---
id: TASK-1906
title: >-
  SEC-11: the pre-push hook never reads or validates git's ref-update stdin, and
  hands that pipe to every command it spawns
status: Done
assignee:
  - TASK-2010
created_date: '2026-08-27 15:39'
updated_date: '2026-08-28 15:13'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:35` (`HOOK_SCRIPT`), `extensions/run-before-push/src/lib.rs:40-47` (`impl_hook_wrappers!`)

**What**: git invokes a `pre-push` hook with two positional arguments (`$1` = remote name, `$2` = remote URL) and writes one line per ref update to the hook's **stdin**:

    <local ref> <local oid> <remote ref> <remote oid>

The installed script is:

    #!/usr/bin/env bash
    exec ops run-before-push

It drops `$1`/`$2` and never reads stdin. Nothing downstream reads it either: `crates/cli/src/subcommands.rs::run_hook_dispatch` goes straight to `run_cmd::run_external_command`, and the runner leaves the child's stdin inherited — explicitly in raw mode (`crates/runner/src/command/exec.rs:509`, `Stdio::inherit()`), and implicitly in the capped path (`spawn_capped`, exec.rs:115, sets only `stdout`/`stderr`, so `stdin` keeps std's inherit default).

Two consequences follow.

1. **The ref-update pipe is handed to arbitrary user-configured commands.** Whatever `[commands.run-before-push]` resolves to inherits git's write end as its stdin. Any step that reads stdin — a tool with a `-`/`--stdin` mode, a `read` in a shell step, a test harness that consumes stdin, an interactive prompt — silently swallows git's ref lines instead of blocking on an empty/terminal stdin, so the same command behaves differently under `git push` than when the developer runs `ops run-before-push` by hand. This is untrusted-ish input crossing a boundary with no validation: the ref names in that stream come partly from the remote.

2. **The hook cannot see what is being pushed**, so it has no way to implement the behaviours the stdin contract exists for: skipping a delete-only push (`local oid` all-zeros), skipping a push with nothing to send, or scoping checks to the pushed range. Today `git push --delete some-branch`, a tag-only push, and an up-to-date no-op push all run the full configured command suite. `run-before-commit` at least has a preflight (`has_staged_files`); `run-before-push` deliberately has `preflight: None` (`crates/cli/src/pre_hook_cmd.rs:27`) because no equivalent probe was written — and the information needed to write one is arriving on the stdin the hook throws away.

**Why it matters**: SEC-11 — external input at a system boundary is neither validated nor bounded, it is forwarded verbatim into child processes. The minimum fix is to stop the leak (`exec ops run-before-push < /dev/null`, or set `Stdio::null()` on the hook path) so a configured command can never consume git's stream. The fuller fix is to read and parse the ref lines in the hook path, validate their shape, and use them for the missing delete-only / nothing-to-push short circuit. Note the leak itself is caused by another crate (the runner's inherited stdin), but the hook script in this crate is where the contract is defined and where the `< /dev/null` guard belongs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 HOOK_SCRIPT (or the run-before-push dispatch path) guarantees no spawned command can read git's pre-push ref-update stream — verified by a test whose configured command reads stdin and observes EOF, not ref lines
- [x] #2 The pre-push ref-update lines are read and their shape validated (four whitespace-separated fields; oids matched against the all-zero sentinel) before any command runs
- [x] #3 A push whose ref updates are all deletions, or that has nothing to push, short-circuits with SUCCESS and a note instead of running the configured commands
- [x] #4 A test covers a delete-only ref line, a normal ref line, and a malformed/empty stdin, asserting the run/skip decision for each
<!-- AC:END -->
