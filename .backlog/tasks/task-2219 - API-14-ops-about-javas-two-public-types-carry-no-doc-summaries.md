---
id: TASK-2219
title: 'API-14: ops-about-java''s two public types carry no doc summaries'
status: Done
assignee: []
created_date: '2026-09-08 07:21'
updated_date: '2026-09-10 16:26'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions-java/about/src/lib.rs
priority: low
ordinal: 127000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/lib.rs:43` (`AboutMavenExtension`), `extensions-java/about/src/lib.rs:68` (`AboutGradleExtension`)

**What**: the crate's entire public surface is two unit structs, and neither has a `///` summary:

```rust
#[non_exhaustive]
pub struct AboutMavenExtension;
…
#[non_exhaustive]
pub struct AboutGradleExtension;
```

The module-level `//!` block mentions them by name, but `cargo doc` renders both type pages with an empty description, and rustdoc's item list shows two bare names. Each also needs a note that it is `#[non_exhaustive]` deliberately (construction goes through the `impl_extension!` factory, not a struct literal) — nothing on the item says so.

Twin instances already filed for the sibling about crates: TASK-2071 (`ops-about`), TASK-2163 (`ops-about-rust`), TASK-2187 (`ops-about-go`). This is the Java one; the file scopes do not overlap.

**Why it matters**: these two names are the only handles a consumer has on the crate, and they are what `stack-java-maven` / `stack-java-gradle` feature users encounter first.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 AboutMavenExtension and AboutGradleExtension each carry a one-line /// summary naming the stack they serve and the manifest files they read
- [x] #2 Each doc notes why the type is #[non_exhaustive] (constructed via the impl_extension! factory)

<!-- AC:END -->
