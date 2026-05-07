---
id: TASK-1025
title: >-
  PATTERN-1: terraform/about find_required_version filters .tf extension
  case-sensitively
status: Done
assignee: []
created_date: '2026-05-07 20:23'
updated_date: '2026-05-07 23:18'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:117`

**What**: The fallback directory walk filters with `p.extension().is_some_and(|e| e == \"tf\")`. The `OsStr` comparison is case-sensitive, so a file like `Versions.TF`, `Main.Tf`, or `provider.TF` is silently skipped.

Terraform itself does not document a case requirement on the extension, and on case-insensitive filesystems (default macOS APFS, Windows NTFS) operators routinely have files committed with mixed-case extensions — git preserves the case the file was added with even though the FS resolves both. The named-candidate list above (`versions.tf`, `main.tf`, `terraform.tf`, `version.tf`) hits the file via case-insensitive FS lookup, but the alphabetical fallback walk does not.

**Why it matters**: Determinism gap with the named-candidate path. A repository whose only constraint lives in `Providers.TF` finds it via the targeted-name list (because the OS resolves case-insensitively when joining `versions.tf`), but a repository whose constraint lives in `Custom.TF` needs the fallback walk and is missed entirely. The About card then advertises "no version" for a project that does declare one.

Suggested fix: lowercase the extension before comparison (`e.to_ascii_lowercase() == \"tf\"` after a `Path::extension` -> `OsStr::to_str` conversion, matching the file-name handling already in this function).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Treat the .tf extension comparison as ASCII-case-insensitive in find_required_version
- [ ] #2 Add a regression test: a versions.TF (uppercase) carrying a constraint in the read_dir fallback is found
<!-- AC:END -->
