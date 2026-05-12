---
id: TASK-1370
title: >-
  ERR-7: main::main() formats anyhow chain via Display, bypassing path-Debug
  escape sweep on inner error fields
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-12 21:34'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:97-105`

**What**: The top-level error printer is `ops_core::ui::error(format!("{e:#}"))` — `{:#}` is the alternate Display form of `anyhow::Error`, which walks the error chain emitting each cause's `Display` representation. The codebase has gone through a sweep (TASK-0944 / TASK-0945 / TASK-1191) to make every tracing event format paths via `?` (Debug) so newline/ANSI/control bytes in a hostile cwd or `.ops.toml` value cannot smuggle ANSI cursor moves or fake log records into operator-facing output. The main error printer is the final UI surface for every error path and does NOT participate in that sweep — any anyhow error whose Display includes a path or attacker-controlled value (e.g. `anyhow!('extension {} not found', name)` with `name = ".[2J./..\u{1b}[31m"`) lands on the operator's terminal verbatim through `ops_core::ui::error`.

**Why it matters**: The path-escape sweep is only as strong as its weakest emitter. `ui::error` itself routes through `sanitise_line` (per SEC-21 / TASK-0981), so this is partially mitigated at the UI layer — but anyhow's chain joiner inserts non-sanitised separators and the inner cause Display strings are passed in verbatim. Confirm the `ops_core::ui::error` pipeline scrubs the final assembled string; if it does, downgrade or close this. If not, the operator-visible end of every failing CLI subcommand is a control-byte injection vector for any anyhow error that interpolated a path or `.ops.toml` value via `%display` instead of `?debug`. Mirrors the rationale in TASK-1340 (extension_cmd) but at the program root.

**Candidates to inspect**:
- `crates/cli/src/main.rs:101` — `ops_core::ui::error(format!("{e:#}"))`
- Verify whether `ops_core::ui::error` runs the full sanitise pipeline on the assembled string, or only on its own log decoration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 verify whether ops_core::ui::error scrubs the final assembled anyhow chain or only emits the wrapped message
- [ ] #2 if scrubbing is incomplete, switch main's printer to a Debug-routed chain walker (e.g. iterate err.chain() with {cause:?}) consistent with the tracing path-escape sweep
<!-- AC:END -->
