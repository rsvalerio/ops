---
id: TASK-1325
title: >-
  API-1: prompt_hook_install returns SUCCESS when user interactively declines
  install but FAILURE when noninteractive, despite both leaving the hook command
  unconfigured
status: Done
assignee:
  - TASK-1382
created_date: '2026-05-11 20:57'
updated_date: '2026-05-12 22:59'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:118-154`

**What**: `prompt_hook_install` is reached when the user invoked `ops run-before-{commit,push}` (without the `install` subcommand) and the corresponding `[commands.<hook-name>]` entry is not yet in `.ops.toml`. The function then chooses an exit code based on *how* the missing configuration is left:

| Mode                                       | User action       | Exit code            |
|--------------------------------------------|-------------------|----------------------|
| Noninteractive (`OPS_NONINTERACTIVE`/`CI`) | n/a (auto-skip)   | `FAILURE` (line 129) |
| Non-TTY                                    | n/a               | `FAILURE` (line 129) |
| Interactive, Confirm = Yes                 | install ran       | `SUCCESS` (line 151) |
| Interactive, Confirm = No                  | declined install  | `SUCCESS` (line 153) |
| Interactive, Ctrl-C / Esc                  | cancelled         | `130`    (line 142)  |

Cases 1, 2, and 4 all end in the same logical state — "the hook command is still not configured; this `ops run-before-{commit,push}` invocation did nothing" — yet case 4 reports SUCCESS while cases 1-2 report FAILURE. From the perspective of a script driver (git running the hook itself, a CI step running `ops run-before-commit`, or a developer scripting `ops` invocations), all three states are identical and should report the same exit code.

The git hook integration is the canonical caller: git treats any nonzero exit from a `pre-commit` script as a veto of the commit. Today, if the user runs the hook via git without ever having configured `[commands.run-before-commit]`, the interactive-decline path returns SUCCESS, so the commit proceeds anyway — the hook is silently inert. That contradicts the noninteractive branch's documented assumption (return FAILURE so the user notices the missing config).

**Why it matters**:
1. Same logical state, different exit codes — a script driver cannot reliably detect "hook ran with no command configured".
2. The interactive-decline SUCCESS path means a user can effectively neutralise the hook by running it once interactively and answering "No", and every subsequent automated invocation will appear to succeed while doing nothing. This is precisely the failure mode the noninteractive FAILURE arm was designed to prevent.
3. The asymmetry is not documented; the only relevant rustdoc is ERR-1 TASK-1189 covering the cancel = 130 case, which is reasonable. Decline = SUCCESS has no rationale comment.

Fix: return `Ok(ExitCode::FAILURE)` (or a dedicated "config missing" sentinel) on the interactive-decline path so the three "still not configured" outcomes are consistent. If a SUCCESS exit really is desired for the user-friendly decline UX, document the reasoning and add a test pinning the contract — but the noninteractive branch's contract should be the one that wins on a hook callable by automation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Return FAILURE (or a sentinel exit code) when the user declines the install prompt, matching the noninteractive branch
- [ ] #2 Document the chosen exit-code contract in the function rustdoc so future contributors can't drift it again
- [ ] #3 Add a unit test that pins the exit code for each of the four 'still not configured' states (noninteractive, non-TTY, decline, cancel)
<!-- AC:END -->
