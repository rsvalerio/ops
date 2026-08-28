---
id: TASK-1868
title: >-
  ARCH-9: EXTENSION_REGISTRY publishes no ordering or name-uniqueness contract,
  so linkme link order decides which extension wins a config_name collision
status: Done
assignee:
  - TASK-1985
created_date: '2026-08-27 15:30'
updated_date: '2026-08-28 19:24'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - crates/extension/src/extension.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:9-16` (`ExtensionFactory`, `EXTENSION_REGISTRY`); cross-crate consumer: `crates/cli/src/registry/discovery.rs:29-52` and `dedup_compiled_extensions`

**What**: `EXTENSION_REGISTRY` is declared as a bare `linkme::distributed_slice` with a two-line doc comment that says only "collecting all extension factories at link time". It publishes:

- **no ordering contract** — `linkme` makes no guarantee about slot order; it is whatever the linker emits, and it can change with link order, LTO settings, or a toolchain bump; and
- **no uniqueness contract** — nothing prevents two factories from returning the same `&'static str` config name, and nothing checks that the returned config name agrees with the `Extension::name()` of the boxed value it is paired with.

Downstream, `crates/cli/src/registry/discovery.rs` walks the slice in slot order (`collect_compiled_extensions`) and then folds the resulting `Vec<(&'static str, Box<dyn Extension>)>` into a `BTreeMap` with **last-write-wins** semantics (`dedup_compiled_extensions`). The `BTreeMap` pins the *iteration* order of the result, which is what its rustdoc reasons about — but it does not pin *which value survives a duplicate key*, because that is decided by the order the pairs arrive in, i.e. by the slice order. That module's own doc comment is explicit that a non-deterministic order at this point would be "genuine functional non-determinism, not just log noise"; the `BTreeMap` closes that hole for the ordering half and leaves the collision half open.

Concretely: if two compiled-in extensions ever claim the same `config_name`, which one is loaded (and which name is reported as `first` vs `second` in the `warn!` breadcrumb) is link-order dependent and can flip between builds without any source change. The `slot` index used in `collect_compiled_extensions`'s debug event is likewise not a stable identifier across builds.

The second identity leak is that `ExtensionFactory` returns `(&'static str, Box<dyn Extension>)` where the `&'static str` config name is an independent value from `Extension::name()`. `dedup_compiled_extensions` keys on the tuple's name, but the duplicate warning, the data-provider audit trail, and `ExtensionInfo` all read `Extension::name()`. A mismatch between the two is silently accepted and produces diagnostics that name a different extension than the one that was actually selected.

**Why it matters**: this crate owns the invariant. Every consumer inherits whatever guarantee the slice declares, and today it declares none, so each consumer has to rediscover the hazard and defend against it independently (the CLI did, partially, and documented the reasoning in a place no extension author reads). Non-deterministic extension selection is the kind of defect that reproduces on one machine and not another.

**Suggested fix**: give the registry a deterministic accessor in this crate rather than exporting the raw slice as the consumption API — e.g. a `pub fn compiled_extensions(config, root) -> Vec<(&'static str, Box<dyn Extension>)>` that probes every factory and sorts the survivors by `(config_name, Extension::name())` before returning, so every consumer inherits the guarantee. At minimum: document the slice order as *unspecified* on `EXTENSION_REGISTRY` itself, document the config-name-vs-`Extension::name()` relationship on `ExtensionFactory`, and state where the collision policy is enforced.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 EXTENSION_REGISTRY's rustdoc states explicitly that linkme slot order is unspecified and must not be relied on for collision resolution or for stable slot indices
- [ ] #2 ExtensionFactory's rustdoc states the relationship between the returned config name and Extension::name(), and what happens when they disagree
- [ ] #3 Duplicate config_name resolution is deterministic across builds — either via a sorting accessor exported from ops-extension, or by a documented rule the CLI consumer enforces before dedup_compiled_extensions runs
- [ ] #4 A test pins the collision outcome: two factories claiming the same config name resolve to the same winner regardless of the order the pairs are supplied in
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
EXTENSION_REGISTRY rustdoc now states slot order is unspecified and slot indices are not stable ids (AC#1). ExtensionFactory rustdoc documents config-name vs Extension::name() and what happens when they disagree (AC#2). New pub fn ops_extension::sort_compiled_extensions imposes a total order on (config_name, Extension::name()); collect_compiled_extensions in crates/cli/src/registry/discovery.rs routes every probed pair through it before dedup_compiled_extensions, so the last-write-wins collision winner is pinned across builds (AC#3). Tests: sort_compiled_extensions_pins_the_collision_winner_regardless_of_input_order (AC#4), sort_compiled_extensions_orders_distinct_config_names.
<!-- SECTION:NOTES:END -->
