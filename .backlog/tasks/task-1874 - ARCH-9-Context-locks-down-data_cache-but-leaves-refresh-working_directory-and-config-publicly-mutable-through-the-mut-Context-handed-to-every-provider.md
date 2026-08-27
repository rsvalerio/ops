---
id: TASK-1874
title: >-
  ARCH-9: Context locks down data_cache but leaves refresh, working_directory
  and config publicly mutable through the &mut Context handed to every provider
status: Triage
assignee: []
created_date: '2026-08-27 15:31'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - crates/extension/src/data.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:386-407` (`Context` fields), `crates/extension/src/data.rs:144` (`DataProvider::provide(&self, ctx: &mut Context)`)

**What**: `Context` was hardened by API-9 / TASK-0349 so that `data_cache` is private — reads go through `Context::cached`, writes only through `Context::get_or_provide` — precisely so "callers cannot bypass the caching/provider contract by inserting raw values directly". The other four fields did not get the same treatment:

```rust
#[non_exhaustive]
pub struct Context {
    pub config: Arc<Config>,
    pub(crate) data_cache: HashMap<String, Arc<serde_json::Value>>,
    pub(crate) in_flight: HashSet<String>,
    pub working_directory: Arc<PathBuf>,
    pub refresh: bool,
    #[cfg(feature = "duckdb")]
    pub db: Option<Arc<dyn DuckDbHandle>>,
}
```

Every `DataProvider` receives `&mut Context`, so every provider — including a provider from a third-party extension crate — can assign to any of those four fields mid-traversal. Two of them are load-bearing:

1. `ctx.refresh = true` (or `= false`) flips the cache-bypass semantics in `get_or_provide` for **every sibling provider that runs later on the same context**. The runner holds one persistent `Context` across repeat queries (per the ERR-1 / TASK-1170 note), so one provider's assignment changes caching behaviour for the rest of the invocation. `with_refresh()` exists as the intended mutator and is a consuming builder method — the public field defeats it.
2. `ctx.working_directory = Arc::new(other)` re-points path resolution for every provider that runs afterwards in the same traversal. A provider that composes other providers via `ctx.get_or_provide(...)` can therefore make the composed provider read from a directory the caller never asked for — a confused-deputy within a single command invocation. Extensions already do real filesystem work relative to this value.

`refresh` is the sharper case because it is a plain `bool` with no invariant attached anywhere; nothing signals that writing it is different from reading it.

**Why it matters**: the encapsulation the type already claims is only half-applied, and the un-encapsulated half is the half that affects other providers rather than just the writer. The cost of closing it is low — these are read-mostly values with obvious accessor shapes — and doing so removes a whole class of "why did provider B see a different cwd" debugging.

**Suggested fix**: make `refresh`, `working_directory`, `config` and `db` private with `&self` accessors (`fn config(&self) -> &Config`, `fn working_directory(&self) -> &Path`, `fn is_refreshing(&self) -> bool`, `fn db(&self) -> Option<&Arc<dyn DuckDbHandle>>`) and keep `with_refresh()` / the constructors as the only mutation paths. `Arc<PathBuf>`'s `Deref` means most existing read sites (`ctx.working_directory.as_path()`, `&ctx.working_directory`) migrate to `ctx.working_directory()` mechanically. If a setter for `db` is genuinely needed by the runner, expose one named method rather than a public field.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Context::refresh and Context::working_directory are no longer publicly assignable; reads go through accessors and the only mutators are the constructors and with_refresh()
- [ ] #2 config and db follow the same treatment, or the rustdoc states explicitly why they must stay publicly assignable by providers
- [ ] #3 Call sites across the workspace's extension crates are migrated to the accessors and the workspace builds clean under the existing clippy policy
- [ ] #4 A test asserts that a provider invoked through Context::get_or_provide cannot change the refresh flag or working directory observed by a sibling provider in the same traversal
<!-- AC:END -->
