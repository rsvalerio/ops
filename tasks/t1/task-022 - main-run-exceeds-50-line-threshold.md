---
id: TASK-022
title: "main.rs::run() exceeds 50-line threshold at ~100 lines"
status: To Do
assignee: []
created_date: '2026-04-08 12:00:00'
labels: [rust-code-quality, CQ, FN-1, low, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/main.rs:78-177`
**Anchor**: `fn run`
**Impact**: The CLI entry point mixes logging initialization, argument preprocessing, help interception, clap parsing, and an 11-arm match dispatch in a single function. While each concern is cleanly separated by whitespace and comments, the function operates at multiple abstraction levels.

**Notes**:
The match dispatch (lines 113-177) is the main contributor to length. Each arm is a clean single-line delegation, and the exhaustive match is idiomatic for clap-based CLIs. This is a borderline FN-1 violation where the CLI entry-point pattern partially justifies the length (similar to state machines and exhaustive match arms noted in FN-1 exceptions).

A minimal improvement would extract the match dispatch to `fn dispatch(subcommand: Option<CoreSubcommand>, ...) -> anyhow::Result<ExitCode>`, reducing `run()` to ~40 lines of setup + dispatch.
