---
id: TASK-020
title: "Extension info extraction (types + commands) duplicated within extension_cmd.rs"
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-code-duplication, CD, DUP-1, DUP-5, low, effort-S, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/extension_cmd.rs:104-114` and `crates/cli/src/extension_cmd.rs:209-219`
**Anchor**: `fn build_extension_row`, `fn print_extension_details`
**Impact**: Eight identical lines for collecting extension type flags and command names are copy-pasted between two functions in the same file. The two copies can drift independently.

**Notes**:
Both functions contain this identical block:
```rust
let mut types = Vec::new();
if info.types.is_datasource() {
    types.push("DATASOURCE".to_string());
}
if info.types.is_command() {
    types.push("COMMAND".to_string());
}

let mut cmd_registry = CommandRegistry::new();
ext.register_commands(&mut cmd_registry);
let commands: Vec<String> = cmd_registry.keys().map(|s| s.to_string()).collect();
```

Additionally, both extract `data_provider` from `info.data_provider_name` with slightly different fallback strings (`unwrap_or_default()` vs `unwrap_or_else(|| "-".to_string())`), which may be an inconsistency.

Fix: extract a helper struct or function, e.g.:
```rust
struct ExtensionSummary {
    types: Vec<String>,
    commands: Vec<String>,
    data_provider: String,
}

fn summarize_extension(ext: &dyn Extension) -> ExtensionSummary { ... }
```
