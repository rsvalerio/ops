---
id: TASK-1340
title: >-
  ERR-7: extension_cmd schema-warn renders anyhow chain via %format!, bypassing
  the path-escape sweep
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 16:38'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:343`

**What**: `print_extension_details` emits the schema-build failure as:

```rust
tracing::warn!(error = %format!("{e:#}"), provider = provider_name, "could not build data registry for schema display");
```

The `%format!("{e:#}")` form pre-stringifies the anyhow chain via Display, and tracing records the resulting `String` verbatim into the `error` field. The anyhow chain transitively includes filesystem paths (workspace_root, .ops.toml, manifest paths) and subprocess stderr captured by downstream extension errors. If any of those carry newlines / ANSI bytes (hostile cwd, attacker-controlled workspace member path), the rendered field forges new log lines exactly like the TASK-0944 / TASK-0945 / TASK-1191 sweep was set up to prevent.

The sibling sites in `init_cmd.rs` (`path = ?path.display()`, `error = %e`) and `registry/discovery.rs` use the Debug-format / direct-`%e` shapes that the sweep standardised on. This is the only site in `crates/cli/src` still on the old `%format!("{e:#}")` shape.

**Why it matters**: ERR-7 log-injection sweep gap. A hostile cwd path or stderr-derived error string can forge log records on the operator-facing `extension show` diagnostic path, identical to the pattern TASK-1191 closed for `init_cmd` warnings. The user-facing `writeln!(w, "\n{msg}")` immediately below is fine — text streams don't forge structured log records — but the structured-log channel does.

<!-- scan confidence: candidates to inspect -->
- crates/cli/src/extension_cmd.rs:343 (only candidate)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extension_cmd.rs:343 no longer renders the error via %format!("{e:#}"); use Debug format (?e) or %e:# so tracing's field recorder controls escaping
- [ ] #2 A test pins that a newline embedded in the error chain reaches the structured log field already escaped (mirroring init_cmd_path_debug_escapes_control_characters)
<!-- AC:END -->
