---
id: TASK-1904
title: >-
  READ-1: types.rs reads the same JSON shape three different ways, and the
  comment justifying one of them is factually wrong
status: Done
assignee:
  - TASK-1999
created_date: '2026-08-27 15:38'
updated_date: '2026-08-28 21:22'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-rust/metadata/src/types.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:311-317` (comment), plus :302, :311, :416, :421, :427, :540, :567, :572, :579, :654, :661

**What**: Three idioms are in use for "read an optional field off a `serde_json::Value`", with no rule separating them:

1. `json_str_with_fallback(self.inner, "name", "")` — the helper (used by `name`, `version`, `id`, `edition`, `manifest_path`, `src_path`, `version_req`, `workspace_root`, `target_directory`)
2. `self.inner["license"].as_str()` — direct `Index` (used by `license`, `repository`, `description`, `rename`, `target`, `source`, `Target::edition`, `doc_path`, and the `kind()` match at :540)
3. `self.inner.get("build_directory").and_then(serde_json::Value::as_str)` — `get` (used by `build_directory` alone)

The `array_iter` / `array_str_iter` extension trait (:39-51) was added precisely to centralise the array version of this idiom "so a future change (e.g. logging when an expected array is missing) lives in one place". The scalar case never got the same treatment, so a change to the missing-field policy — the `tracing::debug!` in `get_or` at :30-33, for instance — reaches only the call sites that happen to use form 1. `get_str_or` (:67) does not even route through `get_or`, so the debug log the trait documents never fires for string fields at all.

The comment that singles out form 3 is incorrect:

```rust
// `get` matches the `Value` index behaviour for a missing key (both
// yield `None` here) without the panic on a non-object `inner`.
```

`serde_json`'s immutable `impl Index<&str> for Value` does not panic on a non-object: it returns `&Value::Null` when the key is absent *or* when the type does not match. Only `IndexMut` panics. So the stated reason for `build_directory` differing from its ten peers does not exist, and a reader who believes it will conclude the other ten accessors are panic-prone — they are not.

**Why it matters**: READ-1. This is a 663-line file whose entire job is field access, so the field-access idiom is its central pattern; three of them plus a comment giving a wrong reason for the odd one out is exactly the ambiguity that makes the next contributor pick a fourth. Low severity because no current behaviour is wrong — the accessors all return the right values today.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One idiom is chosen for optional-scalar field access and applied across all accessors in types.rs, mirroring how array_iter/array_str_iter centralise the array case
- [x] #2 The incorrect comment at types.rs:312-313 is removed or corrected — immutable serde_json Value indexing returns Value::Null, it does not panic on a non-object
- [x] #3 get_str_or routes through get_or (or the divergence is documented), so the missing-field tracing::debug! applies uniformly to string fields
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Closed as obsolete: the three competing optional-scalar field-access idioms, the incorrect "get avoids the panic on a non-object inner" comment at types.rs:312-313, and get_str_or's divergence from get_or all lived in src/types.rs, which TASK-1898 deleted in this same wave (the ARCH-9 decision was "remove the unconsumed surface"). No accessor, no JsonValueExt trait and no json_*_with_fallback helper remains, so there is no idiom left to unify and no comment left to correct. Not implemented - removed.
<!-- SECTION:NOTES:END -->
