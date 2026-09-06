---
id: TASK-1795
title: >-
  ARCH-4: InheritableField/InheritableString/InheritableVec are unnameable
  outside the crate despite being the type of public Package fields
status: Done
assignee:
  - TASK-1994
created_date: '2026-08-27 11:24'
updated_date: '2026-08-28 20:19'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-toml/src/types.rs
  - extensions-rust/cargo-toml/src/inheritance.rs
  - extensions-rust/cargo-toml/src/workspace_root.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:75-78` (the `pub use types::{...}` block), against `extensions-rust/cargo-toml/src/types.rs:167-203`.

**What**: `mod types;` is private. `lib.rs` re-exports `CargoToml, DepSpec, DetailedDepSpec, Package, ParseError, PublishSpec, ReadmeSpec, Workspace, WorkspacePackage` — but **not** `InheritableField`, `InheritableString`, or `InheritableVec`.

`Package` is re-exported and eleven of its public fields are typed `InheritableString` / `InheritableVec`:

```rust
pub struct Package {
    pub version: InheritableString,   // types.rs:108
    pub edition: InheritableString,   // :112
    pub authors: InheritableVec,      // :120
    ...
}
```

So an external consumer can hold a `ops_cargo_toml::Package`, call `p.version.as_str()`, and get a value — but cannot write the type in a signature, cannot `match` on `InheritableField::Value(_) | InheritableField::Inherited { .. }`, and cannot construct a `Package` literal. This is the `unnameable_types` shape (allow-by-default, so nothing in CI reports it). The crate's own tests have to reach around the public surface with `crate::types::InheritableField::...` (`src/tests/types.rs:44`, `src/tests/inheritance.rs:113`, `:182`) — the in-crate symptom of the same gap.

The same applies to `workspace_root::content_declares_workspace` (`workspace_root.rs:325`), which is `pub` inside a private module and reachable only from tests (`src/tests/find_root.rs:2`), and to the five `pub` resolver helpers in `inheritance.rs:99/116/124/131/140`, none of which is re-exported.

**Why it matters**: ARCH-4 — "keep public API re-exports curated in `lib.rs`". Right now the surface is neither curated nor minimal: it is accidentally split between "reachable and nameable", "reachable but unnameable" (the `Inheritable*` family), and "declared `pub` but reachable by nobody" (the resolver helpers, `content_declares_workspace`). Consumers such as `extensions-rust/about/src/identity/resolver.rs`, which builds fallback logic on the `Value` vs `Inherited` distinction, are forced to express it through accessor methods because the variants are not nameable. It also suppresses `clippy::must_use_candidate` on `InheritableField::value` / `as_str` (`types.rs:178`, `:188`), which is why those two accessors lack the `#[must_use]` every other accessor in the file carries.

Decide per item and make it explicit: re-export it, or demote it to `pub(crate)`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 InheritableField, InheritableString and InheritableVec are either re-exported from lib.rs (so every public Package field has a nameable type) or Package's fields no longer expose them
- [x] #2 Each pub item in the private inheritance and workspace_root modules is either re-exported from lib.rs or demoted to pub(crate): resolve_string_field, resolve_vec_field, resolve_optional_string, resolve_readme, resolve_publish, content_declares_workspace
- [x] #3 InheritableField::value and InheritableField::as_str carry #[must_use], consistent with the other accessors in types.rs
- [x] #4 In-crate tests reference the types through the public path rather than crate::types::... where a re-export exists
- [x] #5 cargo clippy passes with the workspace lint policy unchanged (no new -W flags)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-1994.

AC #1: `InheritableField`, `InheritableString` and `InheritableVec` are now re-exported
from lib.rs, so every public `Package` field has a nameable type and consumers can match
on `Value` vs `Inherited` instead of routing through accessors.

AC #2 satisfied by substitution. The literal "demote to `pub(crate)`" option is not
available in this workspace: `clippy::redundant_pub_crate` is on via the workspace
`[workspace.lints]` nursery group and rejects `pub(crate)` inside a private module,
pointing at `pub` instead — which is why those items were spelled `pub` in the first
place. Since `inheritance` and `workspace_root` are private modules, `pub` there is
already crate-internal, so the two options collapse: the decision is "not re-exported".
Made that explicit instead, in the exact place ARCH-4 asks for it — a comment on the
lib.rs re-export block naming `resolve_string_field`, `resolve_vec_field`,
`resolve_optional_string`, `resolve_readme`, `resolve_publish`,
`content_declares_workspace`, `strict_candidate_action` and `CandidateAction` as
deliberately crate-internal, and recording why they stay `pub`.

AC #3: `InheritableField::value` and `as_str` now carry `#[must_use]` (a new `is_absent`
accessor does too).

AC #4: in-crate tests now reference `crate::InheritableField` / `crate::InheritableString`
/ `crate::InheritableVec` / `crate::ReadmeSpec` through the public re-export path rather
than `crate::types::...`.

AC #5: `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
with the workspace lint policy unchanged (no added -W flags, no new allows).
<!-- SECTION:NOTES:END -->
