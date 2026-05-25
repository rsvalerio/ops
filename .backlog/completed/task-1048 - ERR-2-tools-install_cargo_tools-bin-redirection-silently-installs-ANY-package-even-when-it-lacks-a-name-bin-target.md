---
id: TASK-1048
title: >-
  ERR-2: tools install_cargo_tool's --bin redirection silently installs ANY
  package even when it lacks a 'name' bin target
status: Done
assignee: []
created_date: '2026-05-07 20:54'
updated_date: '2026-05-08 06:17'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:47-88` (`install_cargo_tool_with_timeout`)

**What**: When the caller supplies both `name` and `package`, the function emits:

```rust
let mut args = vec!["install"];
if let Some(pkg) = package {
    args.push(pkg);
    args.push("--bin");
    args.push(name);
} else {
    args.push(name);
}
```

The contract is "install package `pkg` and produce binary `name`". `cargo install <pkg> --bin <name>` then fails with `error: no bin target named <name>` when the package does not actually expose that binary, but the failure surface is just a generic `anyhow::bail!("cargo install {} failed", name)` with stderr inherited to the user terminal. There is no preflight check and no structured error indicating *why* the install failed (wrong `--bin`, wrong package, etc.), so the operator sees the cargo error scroll past once and the wrapper reports "cargo install <name> failed" — referencing `name`, not `pkg`, which makes the diagnostic misleading.

A second, related issue lives at the same call site: `install_tool` (lib.rs:118-137) routes `ToolSource::System` *with* a `rustup_component` through the rustup-only branch and silently treats the system-tool half as installed; the same function routes `ToolSource::System` *without* a component to a hard `bail!`. The cargo branch above does not validate the same way — a `ToolSource::Cargo` spec with `package = Some("nonexistent")` calls `cargo install nonexistent` and surfaces only cargo's "not found in registry" error, with the wrapper still bailing on the wrong identifier.

**Why it matters**: ERR-2 — install failure modes are conflated under a single bail string that names the wrong identifier. Operators debugging a misconfigured `[tools]` table cannot tell whether the package name is wrong, the binary name is wrong, or the network probe failed. Mirrors the routing-mismatch finding already filed at TASK-1038 (install_tool routes System+rustup_component through cargo) — same module, adjacent function.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 install_cargo_tool failure includes both package and binary in the error message when both are present (e.g. 'cargo install <pkg> --bin <name> failed')
- [ ] #2 Regression test: install_cargo_tool('does-not-exist', Some('also-missing')) returns an error string that names BOTH identifiers
<!-- AC:END -->
