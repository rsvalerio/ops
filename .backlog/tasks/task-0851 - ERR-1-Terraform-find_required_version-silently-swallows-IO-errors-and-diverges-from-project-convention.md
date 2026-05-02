---
id: TASK-0851
title: >-
  ERR-1: Terraform find_required_version silently swallows IO errors and
  diverges from project convention
status: Triage
assignee: []
created_date: '2026-05-02 09:17'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:68-96`

**What**: Uses `if let Ok(content) = std::fs::read_to_string(...)` and `if let Ok(entries) = std::fs::read_dir(...)`. The Python and Go siblings (lib.rs:81 in extensions-python, go_mod.rs:20) all route file reads through ops_about::manifest_io::read_optional_text, which differentiates NotFound from real errors and tracing::warn!s the latter.

**Why it matters**: A permissions error or transient IO failure on versions.tf is indistinguishable from "no version declared" - a malformed-or-unreadable manifest looks like a missing one. The TASK-0394 invariant cited in the other extensions is broken here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace both read_to_string/read_dir calls with manifest_io::read_optional_text (or an equivalent helper that logs non-NotFound errors)
- [ ] #2 Add a test that injects a non-NotFound error (e.g., a directory named versions.tf) and asserts a tracing::warn! event is recorded
- [ ] #3 Document the fall-back-to-defaults-but-log-on-real-errors rule in the module-level doc comment, matching the other extensions
<!-- AC:END -->
