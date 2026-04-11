---
id: TASK-009
title: "Replace .map_err().unwrap() with .unwrap_or_else() in CLI main"
status: To Do
assignee: []
created_date: '2026-04-07 12:00:00'
labels: [rust-idioms, EFF, ERR-5, low, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/main.rs:109-111`
**Anchor**: `fn main`
**Impact**: The pattern `.map_err(|e: clap::Error| e.exit()).unwrap()` is provably infallible since `clap::Error::exit()` diverges (`-> !`), but using `.unwrap()` obscures this guarantee. Replace with `.unwrap_or_else(|e| e.exit())` which is simpler, communicates intent, and eliminates the `unwrap` call entirely.

**Notes**:
Current code:
```rust
let cli = Cli::from_arg_matches_mut(&mut matches)
    .map_err(|e: clap::Error| e.exit())
    .unwrap();
```

Suggested fix:
```rust
let cli = Cli::from_arg_matches_mut(&mut matches)
    .unwrap_or_else(|e: clap::Error| e.exit());
```

Severity is Low per ERR-5 (provably infallible path). The `unwrap` can never panic because `e.exit()` calls `std::process::exit()` and never returns.
