---
id: TASK-1858
title: >-
  ERR-1: has_manifest_in_dir swallows the read_dir error itself, contradicting
  the breadcrumb policy documented two lines below it
status: To Do
assignee:
  - TASK-1984
created_date: '2026-08-27 15:28'
updated_date: '2026-08-28 14:09'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - crates/core/src/stack/detect.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack/detect.rs:98-101`

**What**: The extension-based detection path discards the directory-level failure with `if let Ok`, immediately above a comment asserting the opposite policy:

```rust
    let extensions = manifest_extensions(stack);
    if !extensions.is_empty() {
        if let Ok(entries) = dir.read_dir() {
            // ERR-1 (TASK-0935): explicit match so a per-entry IO error
            // leaves a `tracing::debug` breadcrumb instead of silently
            // making the manifest "not found".
            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::debug!(
                            parent = ?dir.display(),
                            error = ?e,
                            "stack manifest extension probe: read_dir entry failed; skipping",
                        );
```

Both neighbours honour the policy and this one call does not:

- `manifest_present` (detect.rs:64-78) logs a `tracing::debug!` when `try_exists` fails.
- The per-entry loop (detect.rs:108) logs when an individual `DirEntry` fails.
- The `read_dir` call itself — where the **most likely** error lands — is dropped on the floor.

The likely error is not exotic: EACCES on an unreadable directory during the ancestor walk, which is exactly the scenario the sibling test at `crates/core/src/stack/mod.rs:277` constructs with `chmod 0o000`.

**Why it matters**: Terraform is the only stack using extension-based detection (`manifest_extensions` returns `&["tf"]` for `Stack::Terraform` and `&[]` for everything else), so an unreadable directory makes `has_manifest_in_dir` silently report "no manifest", the walk moves to the parent, and the user gets the wrong stack — or `Generic` — with **zero signal** at any log level. Someone debugging "why does `ops` think my Terraform repo is Generic?" has nothing to go on, which is precisely the outcome TASK-0935 added the per-entry breadcrumb to prevent. The fix is the same explicit `match` already written one level down.

<!-- scan confidence: verified by reading detect.rs:82-122; the `if let Ok(entries)` has no else arm and no other logging covers the read_dir failure -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A failing dir.read_dir() in has_manifest_in_dir emits a tracing::debug breadcrumb naming the directory and the error, matching the per-entry arm below it
- [ ] #2 The path is still treated as 'no manifest here' so the ancestor walk continues — this is a diagnostics fix, not a behaviour change
- [ ] #3 A Unix test makes a directory unreadable and asserts the breadcrumb fires, guarded against the privileged-sandbox case the way the sibling tests in this crate already are
<!-- AC:END -->
