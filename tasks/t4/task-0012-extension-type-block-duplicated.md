---
id: TASK-0012
title: "Extract extension type+commands block duplicated in extension_cmd.rs"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-code-duplication, CD, DUP-1, DUP-5, low, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/extension_cmd.rs:104-119` and `crates/cli/src/extension_cmd.rs:209-224`
**Anchor**: `fn build_extension_row`, `fn print_extension_details`
**Impact**: A 12-line block (build types vector + collect registered commands) is duplicated verbatim within the same file. Both functions compute the same derived data from an extension.

**Notes**:
Duplicated block:
```rust
let mut types = Vec::new();
if info.types.is_datasource() { types.push("DATASOURCE".to_string()); }
if info.types.is_command() { types.push("COMMAND".to_string()); }
let mut cmd_registry = CommandRegistry::new();
ext.register_commands(&mut cmd_registry);
let commands: Vec<String> = cmd_registry.keys().map(|s| s.to_string()).collect();
```

Fix: extract `fn extension_summary(ext, info) -> (Vec<String>, Vec<String>)` returning `(types, commands)`.
