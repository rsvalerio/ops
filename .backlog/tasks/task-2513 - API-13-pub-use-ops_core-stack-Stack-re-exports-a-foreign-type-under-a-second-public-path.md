---
id: TASK-2513
title: 'API-13: pub use ops_core::stack::Stack re-exports a foreign type under a second public path'
status: Done
assignee: []
created_date: '2026-10-10 15:32'
updated_date: '2026-10-10 21:24'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2612'
modified_files:
  - crates/extension/src/extension.rs
priority: low
ordinal: 1000
dedup_key: 'API-13:crates/extension/src/extension.rs:extension'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/extension.rs:6`

**What**: `pub use ops_core::stack::Stack;` lifts a foreign crate's type into `ops_extension`'s public surface. `Stack` already has a public path in its own crate (`ops_core::stack::Stack`), so the type is now reachable by two public paths, doubling every mention in docs, search results, and error messages, and leaving readers unsure whether the two are the same type.

**Why it matters**: API-13 — foreign types come from their own crate; a `pub use` creating a second alias for a type that already has a name is the flag condition. The re-export also buys nothing here: `ops-extension`'s public API already uses five other `ops-core` types without re-exporting them (`Config`, `CommandSpec`, `CommandId`, `AboutFieldDef`, and the `Stack`-bearing `ExtensionInfo::stack` signature), so every extension implementer must already depend on `ops-core` directly. `Stack` is the lone foreign type given an alias, which is the characteristic artifact of iterative refactoring rather than a deliberate umbrella/technical-split decision (the rule's stated exceptions).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Stack is reachable by exactly one public path (ops_core::stack::Stack): the pub use is removed and in-tree usages of ops_extension::Stack are updated to name ops_core::stack::Stack

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
pub use removed; Stack now reachable only as ops_core::stack::Stack. In-tree usages updated beyond the files this task listed, all mechanical path renames: crates/extension/src/{extension.rs,lib.rs,macros.rs} ($crate::Stack → ::ops_core::stack::Stack in impl_extension!), plus ops_extension::Stack → ops_core::stack::Stack in extensions-rust (about, cargo-update, metadata, loc, create-review-tasks, cargo-toml, deps), extensions-{node,java,go,terraform,python}/about, and one grouped use-import in extensions-terraform/about. ops-rust-loc lacked a direct ops-core dep (the only extension crate that did) — added `ops-core = { workspace = true }` to its [dependencies], which is API-13's stated remedy (consumers depend on the owning crate). No in-tree invocation of impl_extension! uses the `stack:` arm, so the macro path change has no current blast radius. Workspace `cargo check --all-targets` clean.
<!-- SECTION:NOTES:END -->
