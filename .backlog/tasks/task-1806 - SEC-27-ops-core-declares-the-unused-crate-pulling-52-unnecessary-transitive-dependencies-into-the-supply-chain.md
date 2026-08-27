---
id: TASK-1806
title: >-
  SEC-27: ops-core declares the unused  crate, pulling 52 unnecessary transitive
  dependencies into the supply chain
status: Triage
assignee: []
created_date: '2026-08-27 11:29'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/Cargo.toml
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/Cargo.toml:15`

**What**: `crates/core/Cargo.toml` declares `config = "0.15"` under `[dependencies]`, but nothing in `crates/core/src` uses it. Every `config::` path in the crate resolves to the crate-local `pub mod config` (`crate::config::...`, `super::config::...`); there is no `use config::…`, no `extern crate config`, and no bare `config::` path that would reach the external crate. Grep verification:

```
$ grep -rnE '(^|[^:a-zA-Z_])config::' crates/core/src --include='*.rs' \
    | grep -v 'crate::config\|super::config\|self::config'
crates/core/src/paths.rs:31://! `config::loader::global_config_path` instead, ...   # doc comment only
```

Cost of the unused dependency, measured with `cargo tree`:

- `cargo tree -p ops-core -e normal` → 80 distinct crates.
- `cargo tree -p config -e normal` → 52 distinct crates.

So roughly two thirds of `ops-core`'s normal dependency tree exists solely for a dependency the crate never calls. `config` 0.15 pulls in multiple format parsers (JSON/YAML/INI/RON/TOML backends) by default, each of which is an independent CVE and unmaintained-crate surface, and each of which lands in `cargo audit` / Trivy output for `ops qa`'s `sec` step.

**Why it matters**: SEC-27/SEC-28 — dependency surface that carries risk without carrying value. Every advisory in those 52 crates becomes an `ops` advisory, every one of them is compiled on every clean build of the workspace's most-depended-on crate, and a reviewer reading the manifest reasonably concludes that `ops-core` layers config sources with the `config` crate when in fact it hand-rolls that in `crates/core/src/config/loader/`.

<!-- scan confidence: verified — grep over all of crates/core/src plus cargo tree measurements above -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The  dependency is removed from crates/core/Cargo.toml, or a concrete in-crate usage is added that justifies keeping it
- [ ] #2 cargo build -p ops-core --all-targets and cargo clippy --all-targets --workspace -- -D warnings both pass after removal
- [ ] #3 cargo tree -p ops-core -e normal shows the transitive crate count drop accordingly, and Cargo.lock is updated in the same change
<!-- AC:END -->
