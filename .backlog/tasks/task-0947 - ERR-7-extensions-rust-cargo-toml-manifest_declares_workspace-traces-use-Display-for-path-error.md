---
id: TASK-0947
title: >-
  ERR-7: extensions-rust cargo-toml manifest_declares_workspace traces use
  Display for path/error
status: Done
assignee: []
created_date: '2026-05-02 16:03'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:382-388, 395-400`

**What**: The debug + warn tracing events in `manifest_declares_workspace` log `path = %path.display(), error = %e` using Display while walking ancestor `Cargo.toml` candidates. Path is attacker-controllable via cloned-repo CWD.

**Why it matters**: Sister to TASK-0930/0937. The TASK-0926 byte-cap fix touches the same function but does not address the formatter. Embedded newlines/ANSI in a Cargo.toml path can forge log lines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Switch path and error fields to Debug formatter at both tracing sites in manifest_declares_workspace
- [x] #2 Regression test asserts escaping of embedded \n/\u{1b} in the candidate Cargo.toml path
<!-- AC:END -->
