---
id: TASK-1742
title: >-
  CL-3: interactive prompts are gated on stdout being a TTY, but inquire renders
  to stderr and reads the controlling terminal
status: To Do
assignee:
  - TASK-1982
created_date: '2026-08-27 11:13'
updated_date: '2026-08-28 14:08'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/cli/src/tty.rs
  - crates/cli/src/theme_cmd.rs
  - crates/cli/src/extension_cmd.rs
  - crates/cli/src/new_command_cmd.rs
  - crates/cli/src/about_cmd.rs
  - crates/cli/src/import_makefile_cmd.rs
  - crates/cli/src/hook_shared.rs
  - crates/cli/src/subcommands.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/tty.rs:64` (`is_stdout_tty`), `crates/cli/src/tty.rs:69-79` (`require_tty` / `require_tty_with`)

**What**: Every interactive command in the crate gates its prompt on `is_stdout_tty()`, which is `std::io::stdout().is_terminal()`:

- `theme_cmd.rs:148` — `require_tty_with("theme select", is_tty)`
- `extension_cmd.rs:299` — `if !is_tty() { bail!("extension show requires an interactive terminal ...") }`
- `new_command_cmd.rs:18` — `require_tty_with("new-command", is_tty)`
- `about_cmd.rs:617` — `require_tty_with("about setup", is_tty)`
- `import_makefile_cmd.rs:47` — `require_tty_with("import-makefile", is_tty)`
- `hook_shared.rs:119` — `require_tty(&format!("{} install", ops.hook_name))`
- `subcommands.rs:139` — `noninteractive_install_blocked` -> `|| !crate::tty::is_stdout_tty()`

The prompts are all `inquire` (0.9.4). Its default crossterm backend builds its terminal from **stderr** (`inquire-0.9.4/src/terminal/crossterm.rs:97`: `io: IO::Std(stderr())`, and `crossterm::execute!(stderr(), ...)` for bracketed paste; the console backend uses `Term::stderr()` likewise) and reads key events from the controlling terminal. **stdout is the one stream inquire never touches.** The gate therefore tests the wrong file descriptor in both directions:

1. `ops theme select > out.txt`, `ops new-command | tee log`, `ops about setup > /dev/null` — stdout is redirected but the terminal is fully attached and the picker would work. All are refused with `... requires an interactive terminal`.
2. `ops theme select 2>/dev/null` (or any invocation with stderr redirected) — stdout is still a TTY, so the gate passes, and inquire then renders the picker into `/dev/null`. The user is left facing an apparently-hung terminal with no prompt, no output, and keystrokes being consumed by a picker they cannot see.

Direction 2 is the damaging one: the guard exists precisely to turn "prompt with nowhere to render" into a clean error, and it does not catch the case where that actually happens.

Note `is_stdout_tty()` is *also* used correctly elsewhere — `subcommands.rs:48` passes it into `ops_about::AboutOptions` to decide colour on the about card, which is rendered to stdout. Only the prompt-gating uses are wrong, so the fix must not simply redefine `is_stdout_tty`.

**Why it matters**: CL-3 / READ-5 — the precondition ("a prompt can be rendered and answered") is encoded against a stream that has nothing to do with either rendering or answering, so the guard is decorative in the failing case and obstructive in the working one. A user cannot script `ops theme select` with output captured, and a user who redirects stderr gets a silent hang instead of the error the guard was written to produce.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tty.rs gains a distinct predicate for prompt capability (e.g. is_prompt_tty) that tests the stream inquire actually uses — stderr, and/or the controlling terminal — rather than stdout
- [ ] #2 require_tty / require_tty_with are switched to the new predicate, and every prompt callsite listed above (theme select, extension show, new-command, about setup, import-makefile, hook install, noninteractive_install_blocked) goes through it
- [ ] #3 is_stdout_tty remains available and is still used for the stdout-rendering decision in subcommands.rs run_about (AboutOptions), with a comment naming why the two predicates differ
- [ ] #4 A unit test pins that a prompt gate refuses when the prompt stream is not a terminal even though stdout is, and permits when the prompt stream is a terminal even though stdout is not — driven through the existing injectable is_tty seam
<!-- AC:END -->
