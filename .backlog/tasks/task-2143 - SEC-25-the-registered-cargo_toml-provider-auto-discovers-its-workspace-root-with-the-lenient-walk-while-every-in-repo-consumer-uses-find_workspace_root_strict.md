---
id: TASK-2143
title: 'SEC-25: the registered cargo_toml provider auto-discovers its workspace root with the lenient walk while every in-repo consumer uses find_workspace_root_strict'
status: Done
assignee: []
created_date: '2026-09-08 07:02'
updated_date: '2026-09-09 18:12'
labels:
  - code-review-rust
  - sec
dependencies: []
parent_task_id: 'TASK-2235'
modified_files:
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-toml/src/workspace_root.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:207`

**What**: `CargoTomlProvider::resolve_root` falls back to `find_workspace_root` (the lenient ancestor walk) when no explicit root was configured:

```rust
find_workspace_root(working_dir).with_context(|| ...)
```

`CargoTomlExtension`'s `register_data_providers` builds exactly that no-root provider for the `linkme` factory (`CargoTomlExtension::new()`), so the `cargo_toml` provider reachable through `DataRegistry`/`Context::get_or_provide`/`query_data` resolves roots leniently. Meanwhile every in-repo consumer that resolves a root itself uses the hardened variant:

- `extensions-rust/about/src/manifest.rs:267` — `find_workspace_root_strict(cwd)`
- `extensions-rust/create-review-tasks/src/provider.rs:41` — `find_workspace_root_strict(ctx.working_directory())`

`find_workspace_root_strict` exists precisely because the lenient walk reaches ancestors via `Path::parent` on the canonicalized start and reads each candidate by its lexical path (TASK-1204, TASK-2026). Its two extra checks — candidate parent must canonicalize onto the start's ancestor chain, candidate `Cargo.toml` must not itself resolve out of that directory — are skipped on the provider path.

**Why it matters**:

1. *Divergent roots for the same cwd.* A candidate the strict walk skips is a candidate the lenient walk accepts (or records as its first-seen fallback), so `ops` answering a `cargo_toml` provider query can target a different workspace root than `about` and `create-review-tasks` compute for the identical working directory. Nothing documents that divergence at either site.
2. *Residual symlink exposure.* `read_capped_to_string` refuses to follow a symlinked file, so a planted symlinked `Cargo.toml` is not read through — but on the lenient path it is not *rejected as a candidate* either: `manifest_declares_workspace` sees the refusal as "no workspace declared", the walk records the directory as its first-seen fallback and can return it as the root, after which `provide_typed` fails on the read. The strict walk skips the candidate and keeps climbing to the real root instead.

Either make the provider use the strict variant (matching the rest of the repo), or state in `resolve_root`'s docs why the data-provider path deliberately opts out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CargoTomlProvider::resolve_root either calls find_workspace_root_strict, or carries a doc comment stating why the provider path deliberately keeps the lenient walk
- [x] #2 A test drives the registered provider (no explicit root) against a tree where the strict and lenient walks disagree, and pins the chosen behavior
- [x] #3 The relationship between the provider's root resolution and about/create-review-tasks' find_workspace_root_strict calls is documented at one place a reader will find

<!-- AC:END -->
