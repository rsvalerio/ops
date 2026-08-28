---
id: TASK-1734
title: >-
  ERR-4: provider payload deserialization errors lose the provider name and the
  failing field path
status: To Do
assignee:
  - TASK-2003
created_date: '2026-08-27 11:12'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions/about/src/providers.rs
  - extensions/about/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/providers.rs:57` (`load_or_default`) and `extensions/about/src/lib.rs:165-167` (`resolve_identity`)

**What**: Both call sites propagate a raw `serde_json` error with `?` and attach nothing:

```rust
// providers.rs:57
Ok(value) => Ok(T::deserialize(value.as_ref())?),

// lib.rs:165
Ok(value) => Ok(<ProjectIdentity as serde::Deserialize>::deserialize(value.as_ref())?),
```

`load_or_default` is generic over `T` and is the single funnel for four subpages — `project_coverage` (`coverage.rs:86`), `project_dependencies` (`deps.rs:44`), `project_units` (`units.rs:65`) — while `resolve_identity` handles `project_identity`. When a stack provider emits a payload whose shape has drifted from `ops_core::project_identity`, the user sees the bare serde message, e.g.:

```
invalid type: string, expected i64
```

with no indication of which provider produced it, which type it was being deserialized into, or which field failed. The function already has `provider: &str` in scope and knows `T`, so the two most useful facts are available and discarded. The surrounding code is otherwise meticulous about diagnosability — every warn in this crate carries `path`/`kind`/`subpage` — which makes this the one path where a real failure is unattributable.

Two layers apply:

- **ERR-4**: `.with_context(|| format!("provider `{provider}` payload did not match {}", std::any::type_name::<T>()))` names the source.
- **ERR-14**: the payload is a nested structure (`ProjectIdentity` carries `languages: Vec<LanguageStat>`; `ProjectCoverage` carries `units: Vec<UnitCoverage>`), so `serde_path_to_error::deserialize` would additionally report `units[3].lines_percent` instead of a message that could refer to any of a hundred fields. That is the difference between a fixable bug report and a bisect.

**Why it matters**: this is the error path a stack-extension author hits while developing a new provider, and it is the least informative message in the crate. The abstraction that was introduced to stop the four subpages drifting (DUP-1 / TASK-0464) also became the point where all four lose their identity in the error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_or_default attaches context naming the provider and the target type before propagating a deserialization failure
- [ ] #2 resolve_identity attaches the same context for the project_identity provider
- [ ] #3 The failing field path is reported (serde_path_to_error or equivalent) for these nested payloads
- [ ] #4 A test registers a provider returning a payload with a wrong field type and asserts the resulting error string contains the provider name and the field path
<!-- AC:END -->
