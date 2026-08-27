---
id: TASK-1750
title: >-
  READ-5: help.rs VALUE_TAKING_GLOBALS hand-mirrors the clap global flags with
  no drift guard
status: Triage
assignee: []
created_date: '2026-08-27 11:14'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/cli/src/help.rs
  - crates/cli/src/args.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:19-23`

**What**:

```rust
/// Global flags declared in `Cli` that take a value as a separate
/// argv entry. Mirrors the `#[arg(long, global = true)]` declarations
/// in `args.rs`; if a new value-taking global is added there it must
/// be listed here too.
const VALUE_TAKING_GLOBALS: &[&str] = &["--tap"];
```

`is_toplevel_help` scans argv itself (it must, because dynamic subcommands cannot be registered before clap parses — see the comment at `main.rs:224-227`), and to do that it needs to know which global flags consume the following argv slot. That knowledge is duplicated by hand from `args.rs`, and the doc comment states the coupling as an instruction to a future maintainer rather than enforcing it.

Nothing detects the drift. The four `--tap` cases in `help.rs` tests (`is_toplevel_help_tap_space_path_then_help_is_toplevel`, `..._not_toplevel`) hardcode `--tap` and would keep passing after a second value-taking global is added. `grep -rn "VALUE_TAKING_GLOBALS\|is_global_set" crates/` returns only the two lines in `help.rs` — there is no test that reconciles the list against `Cli::command()`.

The failure this guards against is not hypothetical; it is the exact bug the constant was introduced to fix (the comment cites PATTERN-1 / TASK-1377). Adding, say, `--log-file <PATH>` as a global would silently regress `ops --log-file out.txt --help`: the path is classified as a positional, `is_toplevel_help` returns `false`, and the user gets clap's plain help instead of the categorized help with dynamic commands — the same symptom `--tap` had.

clap can answer this directly: `Cli::command()` exposes `get_arguments()`, `Arg::is_global_set()`, `Arg::get_long()`, and `Arg::get_num_args()`, so the set is derivable rather than transcribed.

**Why it matters**: READ-5 / CL-3 — an invariant that lives only in a comment ("must be listed here too") is not an invariant. Two declarations of the same fact, in different files, with no mechanism that fails when they disagree, is a latent regression waiting on the next global flag.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The set of value-taking global flags consumed by is_toplevel_help is derived from Cli::command() (global args whose num_args is non-zero) rather than transcribed, OR a test asserts the hardcoded list is exactly equal to that derived set
- [ ] #2 If the list stays hardcoded, the reconciling test fails loudly when a new value-taking global is added to args.rs and not to help.rs, and its failure message names the missing flag
- [ ] #3 Short-flag spellings of value-taking globals are covered as well as long ones, or the test asserts none exist
- [ ] #4 The existing --tap top-level-help tests continue to pass unchanged
<!-- AC:END -->
