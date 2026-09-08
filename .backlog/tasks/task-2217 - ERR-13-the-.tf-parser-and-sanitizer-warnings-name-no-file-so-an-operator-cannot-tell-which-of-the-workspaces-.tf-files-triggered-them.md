---
id: TASK-2217
title: >-
  ERR-13: the .tf parser and sanitizer warnings name no file, so an operator
  cannot tell which of the workspace's .tf files triggered them
status: To Do
assignee:
  - TASK-2236
created_date: '2026-09-08 07:21'
updated_date: '2026-09-08 10:54'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: medium
ordinal: 126000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:232` (`extract_required_version`), `:514` and `:521` (`sanitize_required_version`)

**What**: `find_required_version` probes four named candidates and then walks *every* `.tf` file in the workspace root, feeding each one to `extract_required_version`. Three warnings fire from inside that per-file pipeline with no path in the event:

```rust
tracing::warn!("unbalanced closing brace in .tf content; skipping file");
tracing::warn!(len = value.len(), "required_version contains control characters; dropping the value");
tracing::warn!(original_len = ..., cap = ..., "required_version value exceeds cap; truncating before rendering");
```

Every other warn in this crate carries the path it is about (`root = ?root.display()`, `modules_dir = ...`, `module_dir = ...`, `entry = ...`), and `ops_about::manifest_io::read_optional_text` is handed a `kind` argument for exactly this reason — `find_required_version` already computes one (`"<unnamed>.tf"` fallback at `:113`) and then throws it away before calling `extract_required_version`.

**Why it matters**: the control-character warn is the SEC-11 signal that an unaudited checkout tried to smuggle an ANSI payload into the About card. It is the one warn an operator would actually investigate, and it does not say which file to look at — in a repo with a dozen root `.tf` files that is a manual grep. The same is true of the malformed-brace warn, which is emitted once per bad file during the fallback walk with nothing to distinguish the repeats.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_required_version (or its callers) attach the source file path/kind to the unbalanced-brace warning
- [ ] #2 sanitize_required_version's control-character and truncation warnings identify the .tf file the value came from
- [ ] #3 The path is passed as a named structured tracing field, consistent with the existing root/modules_dir/entry fields in this crate
- [ ] #4 A test asserts the control-character warning names the offending file
<!-- AC:END -->
