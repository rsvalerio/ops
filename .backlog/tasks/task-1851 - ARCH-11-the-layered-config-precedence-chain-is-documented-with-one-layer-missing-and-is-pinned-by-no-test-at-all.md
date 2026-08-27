---
id: TASK-1851
title: >-
  ARCH-11: the layered-config precedence chain is documented with one layer
  missing and is pinned by no test at all
status: Triage
assignee: []
created_date: '2026-08-27 15:25'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - crates/core/src/config/mod.rs
  - crates/core/src/config/loader/mod.rs
  - crates/core/src/config/loader/env.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:3` (the documented order), `crates/core/src/config/loader/mod.rs:213-227` (the real chain), `crates/core/src/config/loader/env.rs:95-141` (the entire env test module)

**What**: Two halves of the same gap.

**1. The documented order omits `.ops.d`, which has the highest file precedence.**

```rust
//! Resolution order: internal default → global config → local `.ops.toml` → env vars.
```

The actual chain is five layers, and the undocumented one *overrides* `.ops.toml`:

```rust
global::load_global_config(&mut config).context("loading global config")?;

let local_path = workspace_root.join(".ops.toml");
if let Some(overlay) = read_config_file(&local_path)... { merge_config(&mut config, overlay); }

conf_d::merge_conf_d(&mut config, workspace_root)...;   // <-- overrides .ops.toml, undocumented

env::merge_env_vars(&mut config)...;
```

(The same omission is repeated in `README.md:71`, and `docs/components.md:286` additionally names the env prefix as `CARGO_OPS_*` rather than `OPS__` — both outside this crate's scope, but they establish the drift is systemic rather than a stray typo.)

**2. No test pins any of it.** `env.rs`'s test module tests only the `scan_ops_env_keys` helper:

```rust
    fn scan_ops_env_keys_zero_when_only_utf8_keys() {
        // The harness env may already carry OPS__ vars from prior tests; this
        // assertion only pins the non-UTF-8 counter, not the presence flag.
        let (_, non_utf8) = scan_ops_env_keys();
        assert_eq!(non_utf8, 0, "no non-UTF-8 OPS__ keys expected in baseline env");
    }
```

A grep for `OPS__` across `crates/**/*.rs` returns only doc comments and those two helper tests — **there is no test that `merge_env_vars` applies an override at all**, none that `.ops.d` beats `.ops.toml`, and none that env beats `.ops.d`. `load_config_local_parse_error_names_layer` pins an error breadcrumb, not precedence.

**Why it matters**: the entire `OPS__` overlay hangs on an undocumented `config-rs` 0.15 detail — `prefix_separator` defaults to `separator`, so the stripped prefix is `ops__` (`config-0.15.25/src/env.rs:245-255`). If a future `config-rs` bump restores a `_` default, `OPS__OUTPUT__THEME` stops matching, `env_config` deserializes to an all-`None` `ConfigOverlay`, and `merge_config` becomes a **silent no-op**: every operator's CI override vanishes with no error, and the test suite stays green. It works today (verified: `OPS__OUTPUT__THEME=compact ops hi` does switch themes), which is precisely why it needs pinning rather than assuming.

The same gap means reordering the four merge calls at loader/mod.rs:213-227 — the one guarantee the layered loader exists to provide — is caught by nothing, and a reader who trusts the module doc will place a setting in `.ops.toml` expecting it to win over `.ops.d`.

<!-- scan confidence: verified by reading; grep-exhaustive for `OPS__` and `.ops.d` across crates/**/*.rs, and the env override behaviour was confirmed against the built binary -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The module doc at config/mod.rs states all five layers in their real precedence order, including .ops.d between .ops.toml and env vars
- [ ] #2 A test sets an OPS__ variable and asserts through the real load path that the value actually overrides the file layer, so a config-rs prefix-separator change fails the suite instead of silently no-opping
- [ ] #3 A test asserts .ops.d/*.toml overrides a conflicting key in .ops.toml
- [ ] #4 A test asserts the env layer overrides a conflicting key set in .ops.d
<!-- AC:END -->
