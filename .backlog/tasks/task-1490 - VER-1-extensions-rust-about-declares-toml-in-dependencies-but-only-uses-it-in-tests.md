---
id: TASK-1490
title: >-
  VER-1: extensions-rust/about declares toml in [dependencies] but only uses it
  in tests
status: To Do
assignee:
  - TASK-1578
created_date: '2026-05-18 16:43'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - version
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/Cargo.toml:20`

**What**: `toml = { workspace = true }` is declared in `[dependencies]`, but every `toml::` use in the crate sits inside `#[cfg(test)]` modules:

- `src/identity/resolver.rs:70` — `toml::from_str(toml_str).expect("test toml should parse")` (inside `mod tests`)
- `src/query.rs:1146`, `1163`, `1230` — all inside `mod tests`

No production code in `src/lib.rs`, `src/deps_provider.rs`, `src/units.rs`, `src/coverage_provider.rs`, `src/query.rs`, `src/identity/*.rs` references `toml::`. Production TOML parsing is delegated to `ops_cargo_toml::CargoToml::parse`.

**Why it matters**: Test-only deps in `[dependencies]` enlarge the runtime dependency closure for downstream consumers (extra crate compilation, extra `Cargo.lock` entries, larger feature-resolution surface). It also misrepresents the crate's API contract: a reader auditing the crate would assume `toml` is needed at runtime. Sister crates in the workspace (and the workspace's ARCH-11 policy) keep test-only deps in `[dev-dependencies]`.

<!-- scan confidence: verified via `grep -nE 'toml::' src/**` returns only test-module call sites -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move `toml` from `[dependencies]` to `[dev-dependencies]` in extensions-rust/about/Cargo.toml
- [ ] #2 Confirm `cargo build -p ops-about-rust` and `cargo test -p ops-about-rust` both pass
- [ ] #3 Confirm no production code references `toml::` after the move
<!-- AC:END -->
