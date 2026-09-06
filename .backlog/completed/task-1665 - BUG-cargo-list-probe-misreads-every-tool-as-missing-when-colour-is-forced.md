---
id: TASK-1665
title: >-
  BUG: cargo --list probe misreads every tool as missing when colour is forced
status: Done
assignee: []
created_date: '2026-08-15 00:00'
updated_date: '2026-08-15 00:00'
labels:
  - bug
  - extensions
dependencies:
  - TASK-1664
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/cargo.rs`

**What**: `check_cargo_tool_installed` and `capture_cargo_list` shell out to
`cargo --list` and parse the result by splitting each line on whitespace and
comparing the first token to the subcommand name. Neither passed `--color
never`, and the parser did not strip ANSI escapes.

When colour is forced — `CARGO_TERM_COLOR=always` in the environment — every
entry arrives wrapped:

```
    \x1b[1m\x1b[96madd                 \x1b[0m Add dependencies to a Cargo.toml manifest file
```

The first whitespace-delimited token is then `\x1b[1m\x1b[96madd`, not `add`, so
**no entry ever matches** and every cargo tool is reported as not installed.

**Why it matters**: this is a production bug, not a test artifact. Any user with
`CARGO_TERM_COLOR=always` exported gets wrong answers from `ops` about which
cargo tools are installed. It is not an exotic setting — this repository's own
CI sets it at the top of `ci.yml`.

**How it was found**: it is exactly the failure that surfaced when TASK-1664
made CI run the full test suite instead of only ignored tests. The drift test
`cargo_builtins_list_is_in_sync` had been failing invisibly; the check had never
run it. It passes locally because an interactive shell leaves colour on
auto-detect, and cargo disables colour when stdout is not a TTY — so the bug is
invisible on a workstation and live on CI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The probe returns correct results with `CARGO_TERM_COLOR=always` set
- [x] #2 A regression test covers colourised `cargo --list` output
- [x] #3 The fix does not depend on the caller's environment
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in two independent layers; either alone resolves the bug, and both are
kept deliberately.

**1. Suppress colour at the source.** Both invocation sites now pass
`--color never`:

```rust
cmd.args(["--color", "never", "--list"]);
```

This makes the probe independent of the caller's environment rather than
relying on cargo's TTY auto-detection (AC #3), which is what made the bug
invisible locally in the first place.

**2. Harden the parser.** `is_in_cargo_list` strips ANSI CSI sequences from the
token before comparing. This matters because the function is reachable with
arbitrary stdout — notably through the public `capture_cargo_list` — so it
should not assume the caller suppressed colour. `strip_ansi` borrows unchanged
when there is no escape present, so the normal path allocates nothing.

**Regression test**: `is_in_cargo_list_is_not_fooled_by_ansi_colour_codes`
asserts on the parser rather than on a subprocess, so it holds regardless of
which cargo is installed on the machine running the tests. It was written
before the parser fix and observed failing, then passing after — it is not
vacuous.

Verified under CI's exact environment: `CARGO_TERM_COLOR=always cargo test -p
ops-tools --lib probe::cargo` passes both tests. Clippy clean.
<!-- SECTION:NOTES:END -->
