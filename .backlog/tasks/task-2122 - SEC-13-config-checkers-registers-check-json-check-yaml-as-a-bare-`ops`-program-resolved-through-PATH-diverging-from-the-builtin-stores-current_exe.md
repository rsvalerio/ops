---
id: TASK-2122
title: >-
  SEC-13: config-checkers registers check-json/check-yaml as a bare `ops`
  program resolved through PATH, diverging from the builtin store's
  current_exe()
status: To Do
assignee:
  - TASK-2234
created_date: '2026-09-08 06:54'
updated_date: '2026-09-08 10:53'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
  - crates/runner/src/command/builtins.rs
priority: medium
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:59-72`

**What**: `register_commands` registers both checkers as
`ExecCommandSpec::new("ops", ["check-json"])` / `["check-yaml"]` — a **bare
program name**, which `std::process::Command` resolves through `PATH`.

The same two command ids are also registered by the runner's builtin store,
`crates/runner/src/command/builtins.rs:27-54`, which deliberately does the
opposite: it resolves `std::env::current_exe()` and only falls back to the
literal `"ops"` when that lookup fails, then sets `display_program =
Some("ops")` so the step line still *renders* as `ops <subcommand>`. Its
module doc states the intent explicitly ("prefer `std::env::current_exe`
(absolute, robust under renamed/aliased shells) and fall back to `"ops"` when
the current-exe lookup fails").

So the crate has two registrations of the same commands with divergent
program resolution, and the extension's is the unsafe half. `PATH` is
inherited from the invoking environment and is not cleared anywhere on this
path (no `.env_clear()`), so whichever `ops` sits earliest on `PATH` is the
binary that runs the validation.

Same pattern in the sibling extensions (`extensions/text-fixers/src/lib.rs:95,101`,
`extensions/about/src/lib.rs:69`); this task is scoped to config-checkers, the
crate under review, but the fix should be a shared helper rather than three
copies.

**Why it matters**: two distinct problems from one line.

1. *Integrity* (SEC-13): "a bare program name is resolved through `PATH`, so
   pass an absolute path for anything security-relevant." A checker that
   gates a pre-commit hook and drives the process exit code is
   security-relevant: a `ops` shim anywhere earlier on `PATH` (a stale
   `~/.cargo/bin/ops`, a `./node_modules/.bin`-style entry, a
   direnv-injected dir, a CI image with a vendored copy) silently becomes the
   validator, and it can exit 0 on files that do not parse. Nothing in the
   run output distinguishes this — `display_program` renders `ops` either
   way.
2. *Version skew* (correctness): even without an attacker, a developer
   running a freshly-built `./target/debug/ops` gets their **installed** `ops`
   for the checker step, so `check-json` validates with different limits
   (`MAX_NESTING_DEPTH`, `MAX_EXPANDED_NODES`, `DEFAULT_MAX_BYTES`) than the
   binary the user invoked. That is exactly the failure `builtins.rs` chose
   `current_exe()` to avoid, and the extension re-opens it.

<!-- scan confidence: verified by reading both registration sites -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 check-json and check-yaml registered by ConfigCheckersExtension spawn an absolute path derived from std::env::current_exe(), falling back to the literal "ops" only when current_exe() fails
- [ ] #2 The rendered step line still reads 'ops check-json' / 'ops check-yaml' (display_program), so user-visible output is unchanged
- [ ] #3 The current_exe-then-fallback resolution is factored into one shared helper used by both crates/runner/src/command/builtins.rs and the extension registration, so the two registrations of the same command id cannot diverge again
- [ ] #4 A test asserts the extension-registered spec's program is absolute when current_exe() succeeds
<!-- AC:END -->
