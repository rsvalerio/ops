---
id: TASK-1879
title: >-
  TRAIT-4: DataRegistry and Context have no Debug impl, so no downstream type
  holding one can derive Debug
status: To Do
assignee:
  - TASK-1985
created_date: '2026-08-27 15:32'
updated_date: '2026-08-28 14:09'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - crates/extension/src/data.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:174-185` (`DataRegistry`), `crates/extension/src/data.rs:386-407` (`Context`), `crates/extension/src/data.rs:114-161` (`DataProvider`), `crates/extension/src/data.rs:364-370` (`DuckDbHandle`)

**What**: `DataRegistry` derives only `Default`; `Context` derives nothing. Both are core public types of the framework crate, and both appear in the signature every extension author implements (`fn provide(&self, ctx: &mut Context)`, `fn register_data_providers(&self, registry: &mut DataRegistry)`). Their sibling `CommandRegistry` does derive `Debug` (`extension.rs:90`), and every other public type in the crate — `DataField`, `DataProviderSchema`, `ExtensionType`, `SharedError`, `DataProviderError` — has `Debug` too. These two are the exceptions.

The blocker is real but narrow: `Box<dyn DataProvider>` and `Arc<dyn DuckDbHandle>` are not `Debug`, so `#[derive(Debug)]` will not compile. That is a reason to write the impl by hand, not a reason to have none. `DataRegistry` can print its provider names (it already computes them for `provider_names()`); `Context` can print the cwd, the refresh flag, the cached keys, the in-flight keys, and whether a db handle is attached.

The `Debug` bound is also absent from the `DataProvider` trait itself, so an extension author cannot get a useful representation of a provider either — `dropped_provider_reports_name = %provider.name()` in `register` exists precisely because there was nothing better to log.

**Why it matters**: `Debug` is part of a type's public API. Any downstream struct that holds a `Context` or a `DataRegistry` — a test harness, an extension's internal state, a wrapper type — cannot `#[derive(Debug)]`, and the omission propagates outward to everything that holds *that*. It also removes the two types from `dbg!`, from `assert_eq!` failure output, from `tracing`'s `?field` syntax (which this codebase deliberately prefers for untrusted values — see the SEC-21 / TASK-1226 note a few lines above `Context`), and from panic messages in tests. `clippy::missing_debug_implementations` is the mechanical form of this rule; TRAIT-4 is explicit that `Debug` is the one trait to derive by default unless the type carries secrets, which neither of these does.

**Suggested fix**: hand-write `impl Debug` for both. For `DataRegistry`, a `debug_struct("DataRegistry")` listing the provider names in registration order plus the pending `duplicate_inserts`. For `Context`, list `working_directory`, `refresh`, the `data_cache` keys (not the values — they are provider output that may be large), the `in_flight` keys, and `db: Some/None`. Consider adding `Debug` as a supertrait bound on `DataProvider` so registry Debug output can name concrete provider types rather than only their registered keys — evaluate whether that is a breaking change worth taking for downstream implementers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DataRegistry implements Debug, printing provider names in registration order and any pending duplicate_inserts entries
- [ ] #2 Context implements Debug, printing working_directory, refresh, cached keys, in-flight keys and whether a db handle is attached — keys only, never cached provider values
- [ ] #3 A test asserts the Debug output of each type names the entries it holds, so the impls do not silently rot into an empty struct
- [ ] #4 Whether DataProvider should gain a Debug supertrait bound is decided and the decision recorded in the trait's rustdoc
<!-- AC:END -->
