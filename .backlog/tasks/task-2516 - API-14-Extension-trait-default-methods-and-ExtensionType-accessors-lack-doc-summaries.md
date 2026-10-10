---
id: TASK-2516
title: 'API-14: Extension trait default methods and ExtensionType accessors lack doc summaries'
status: To Do
assignee: []
created_date: '2026-10-10 15:33'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2612'
modified_files:
  - crates/extension/src/extension.rs
priority: medium
ordinal: 1000
dedup_key: 'API-14:crates/extension/src/extension.rs:Extension'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:97` (`ExtensionType::is_datasource`), `:101` (`is_command`), `:325` (`Extension::description`), `:329` (`shortname`), `:333` (`types`), `:337` (`command_names`), `:341` (`data_provider_name`), `:345` (`stack`), `:349` (`info`), `:252` and `:260` (`IntoIterator::into_iter` impls)

**What**: Public items with no doc summary on the crate's central interface. The `Extension` trait is what every extension crate in this workspace implements, yet seven of its ten methods (`description`, `shortname`, `types`, `command_names`, `data_provider_name`, `stack`, `info`) have no doc comment at all — only `name`, `register_commands` and `register_data_providers` are documented. The `ExtensionType::is_datasource` / `is_command` accessors and the two `IntoIterator::into_iter` impls are likewise undocumented. (The pub fields on `ExtensionInfo` rely on the type-level doc, which is acceptable but thin.)

**Why it matters**: API-14 — the summary sentence on a public item is mandatory, and for a library crate the trait docs are the product: `cargo ops data info`, IDE hover, and rustdoc for extension authors all render exactly this surface. The workspace lints table does not enable `missing_docs`, so nothing catches these mechanically; the gaps are also uneven (neighbouring items are documented), which shows they are oversights rather than policy. Note the ordering/return contracts these methods carry (e.g. `info()` aggregating the other accessors, `command_names` mirroring what `register_commands` inserts) are exactly the kind of behaviour a one-line summary should state.

<!-- scan confidence: candidates to inspect — list above verified by reading each item; all are production (non-test) public API -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every public item listed above carries a doc comment whose first paragraph is a ~15-word summary
- [ ] #2 Extension trait method docs state their default value and when an extension should override them
<!-- AC:END -->
