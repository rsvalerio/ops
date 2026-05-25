---
id: TASK-1380
title: >-
  PERF-1: run_extension_show enumerates compiled-in extensions twice per
  invocation
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 21:52'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:236-297`

**What**: `run_extension_show_with_tty_check` calls `collect_compiled_extensions(config, &cwd)` at line 252 to resolve the requested extension and run the interactive picker. It then passes the resolved extension to `print_extension_details`, which at line 334 calls `build_data_registry(config, cwd)` for the schema lookup. `build_data_registry` (registration.rs:259) internally calls `builtin_extensions(config, workspace_root)`, which calls `collect_compiled_extensions` again — a second full walk of `ops_extension::EXTENSION_REGISTRY` with each factory re-invoked (and each factory's prerequisite probes re-run, including I/O for the `git`/`tools`/`metadata` extensions).

By contrast, `setup_extensions` in `run_cmd.rs:308-318` already does this correctly: one `builtin_extensions` call up front, the resulting `Vec` flowed into both `register_extension_commands` and `register_extension_data_providers`. The `extension show` path inherited an older shape that pre-dates the `as_ext_refs` helper.

**Why it matters**: most factories declined (returning None for the wrong stack / missing tool) cost only a `std::process::Command` probe, but `extension show` is invoked interactively by operators inspecting a slow stack — the doubled work is observable as visible UI latency on `git status` / `cargo --version` style probes. Also doubles the per-process `tracing::debug!` 'extension factory declined to construct' breadcrumbs (discovery.rs:44) when `RUST_LOG=ops=debug` is set, making the log harder to read.

**Fix**: have `print_extension_details` accept an `&DataRegistry` already built by the caller (or accept the pre-collected `&[Box<dyn Extension>]`), so the second enumeration goes away. Mirrors the `setup_extensions` shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_extension_show invokes collect_compiled_extensions / builtin_extensions at most once per call
- [ ] #2 build_data_registry is either threaded a pre-built ext list or replaced by a registry-reuse helper
- [ ] #3 Existing extension_show tests still pass and a new test pins the single-enumeration contract
<!-- AC:END -->
