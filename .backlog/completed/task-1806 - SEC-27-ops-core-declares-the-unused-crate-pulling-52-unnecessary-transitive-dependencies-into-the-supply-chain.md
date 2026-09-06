---
id: TASK-1806
title: >-
  SEC-27: ops-core declares the unused  crate, pulling 52 unnecessary transitive
  dependencies into the supply chain
status: Done
assignee:
  - TASK-1983
created_date: '2026-08-27 11:29'
updated_date: '2026-08-28 23:51'
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
- [x] #1 The  dependency is removed from crates/core/Cargo.toml, or a concrete in-crate usage is added that justifies keeping it
- [x] #2 cargo build -p ops-core --all-targets and cargo clippy --all-targets --workspace -- -D warnings both pass after removal
- [x] #3 cargo tree -p ops-core -e normal shows the transitive crate count drop accordingly, and Cargo.lock is updated in the same change
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Resolved during TASK-1983 (wave149) as a **no-op with an AC substitution**: the
finding's premise does not hold. `config` is a live dependency of `ops-core` —
`crates/core/src/config/loader/env.rs:9` does `use config as config_crate;` and
builds the whole `OPS__*` environment overlay on
`config_crate::Environment::with_prefix("OPS")`. The grep in the description
(`(^|[^:a-zA-Z_])config::`) cannot see it because the crate is imported under an
alias, so no bare `config::` path ever appears.

AC #1 is satisfied by its own second branch ("a concrete in-crate usage ...
that justifies keeping it"): the usage already exists and is now documented at
the `[workspace.dependencies]` declaration so the next SEC-27 sweep does not
re-file this. AC #2 passes (`ops verify` clean). AC #3 is obsolete — there is no
transitive-count drop to observe, because nothing was removed; `Cargo.lock` is
unchanged, which is the correct outcome here.

Related change landed in the same wave: `config` moved from an inline
`config = "0.15"` pin in `crates/core/Cargo.toml` to `[workspace.dependencies]`
alongside `strum` (TASK-1807).
<!-- SECTION:NOTES:END -->
