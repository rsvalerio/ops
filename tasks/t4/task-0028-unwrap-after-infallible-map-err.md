---
id: TASK-028
title: "Unnecessary unwrap after infallible map_err in run()"
status: Triage
assignee: []
created_date: '2026-04-09 20:30:00'
labels: [rust-idioms, EFF, ERR-5, low, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/main.rs:109-111`
**Anchor**: `fn run`
**Impact**: The pattern `.map_err(|e: clap::Error| e.exit()).unwrap()` is provably infallible because `clap::Error::exit()` returns `!` (never type), so the `Err` branch never reaches `unwrap()`. While safe, the `unwrap()` is dead code that obscures intent. Replace with `unwrap_or_else(|e| e.exit())` which is semantically identical, eliminates the intermediate `map_err`, and makes the "exit on error" intent explicit without a dangling `unwrap()`.

**Notes**:
Current code:
```rust
let cli = Cli::from_arg_matches_mut(&mut matches)
    .map_err(|e: clap::Error| e.exit())
    .unwrap();
```
Suggested:
```rust
let cli = Cli::from_arg_matches_mut(&mut matches)
    .unwrap_or_else(|e: clap::Error| e.exit());
```
Severity is Low because the code is correct and the `unwrap()` can never panic. The improvement is purely readability — removing a call that appears fallible but isn't.
