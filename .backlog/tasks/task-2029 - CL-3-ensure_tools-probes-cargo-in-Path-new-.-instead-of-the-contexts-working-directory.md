---
id: TASK-2029
title: >-
  CL-3: ensure_tools probes cargo in Path::new(".") instead of the context's
  working directory
status: Done
assignee:
  - TASK-2050
created_date: '2026-08-28 20:51'
updated_date: '2026-08-29 13:26'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/deps/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:79-81` (`check_tool`), `:113-118` (`ensure_tools`)

**What**: `check_tool` hardcodes the probe's working directory:

```rust
fn check_tool(tool: &CargoTool) -> anyhow::Result<()> {
    check_tool_in(tool, std::path::Path::new("."))
}
```

`check_tool_in` already takes a `working_dir` — the seam exists and is used by the timeout test — but the production caller ignores it and passes the process CWD. Every other directory-sensitive call in this crate goes through the context: `DepsProvider::provide` uses `ctx.working_directory()` for both `run_cargo_upgrade_dry_run` and `run_cargo_deny`, and `build_user_context` loads `.ops.toml` from `std::env::current_dir()`.

Today the two coincide because `run_deps` calls `ensure_tools()` before building the context and `ops deps` runs from the CWD. They stop coinciding the moment `ensure_tools` is called from anywhere with an explicit root — an embedding of the extension, a future `--path` flag, or a test that wants to probe inside a tempdir without `chdir`-ing the whole process. The last one is not hypothetical: `tests::command_path_tests` (TASK-1845) has to chdir under `#[serial]` purely because `ensure_tools` cannot be pointed at a directory.

**Why it matters**: a probe that resolves against the process CWD rather than the directory the command is operating on is the same class of divergence TASK-1762 filed for the Rust about providers. It is invisible while they agree and produces a wrong answer — "cargo deny is not installed", or a probe run against an unrelated workspace's toolchain config — as soon as they do not. Threading the directory also removes the process-global `chdir` from the command-path tests.

**Origin**: discovered during TASK-1997 while fixing TASK-1845, which names the divergence but does not carry an acceptance criterion for it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 check_tool / ensure_tools take the working directory from the caller instead of hardcoding Path::new(".")
- [x] #2 run_deps passes the context's working directory (or the cwd it resolved) so the probe and the collection calls agree on one directory
- [x] #3 The command-path tests can point ensure_tools at a tempdir without chdir-ing the process
<!-- AC:END -->
