---
id: TASK-019
title: "expect() in Stack::default_commands panics instead of returning Result"
status: To Do
assignee: []
created_date: '2026-04-08 12:00:00'
labels: [rust-idioms, EFF, ERR-5, low, effort-S, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/stack.rs:128-129`
**Anchor**: `fn default_commands`
**Impact**: `Stack::default_commands()` uses `.expect()` to parse embedded TOML, which would panic if any stack's default config file were malformed. While provably infallible today (the TOML is embedded via `include_str!` and validated by tests), this is a public library method — callers cannot handle failures gracefully. The crate's own `config/mod.rs:313` parses the same embedded default TOML using `?` with `.context()`, making this an inconsistency.

**Notes**:
Current code:
```rust
let config: Config =
    toml::from_str(toml).expect("stack default commands TOML must be valid");
```

Suggested fix — change return type to `anyhow::Result<IndexMap<String, CommandSpec>>`:
```rust
pub fn default_commands(&self) -> anyhow::Result<IndexMap<String, CommandSpec>> {
    let toml = match self.default_commands_toml() {
        Some(t) => t,
        None => return Ok(IndexMap::new()),
    };
    let config: Config =
        toml::from_str(toml).context("stack default commands TOML must be valid")?;
    Ok(config.commands)
}
```

Severity is Low per ERR-5 (known-literal parse of compile-time constants). The `expect` message is descriptive, mitigating debuggability concerns.
