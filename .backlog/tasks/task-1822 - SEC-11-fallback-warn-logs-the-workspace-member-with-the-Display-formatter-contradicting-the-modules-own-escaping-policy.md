---
id: TASK-1822
title: >-
  SEC-11: fallback warn logs the workspace member with the Display formatter,
  contradicting the module's own escaping policy
status: To Do
assignee:
  - TASK-1996
created_date: '2026-08-27 11:33'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:32-35`

**What**: The fallback breadcrumb inside `provide` interpolates the member string with the `%` (Display) formatter:

```rust
tracing::warn!(
    member = %member,
    "no package name for workspace member; falling back to display name"
);
```

`member` is a raw `[workspace].members` entry read out of the project's `Cargo.toml`, i.e. content this process does not control. Rendered with `%` it reaches the log verbatim, embedded newlines and ANSI escapes included, so a member entry can inject additional lines into the log stream and forge a log record.

This directly contradicts the policy the file states 20 lines below it, in the doc comment on `member_package_name` (provider.rs:53-56): "tracing path fields use the `?` formatter so attacker-controlled member names cannot forge log records". `member_package_name` itself honours the policy (`path = ?member_toml.display()`, `error = ?e` at provider.rs:62-67 and 73-78); only the call site in `provide` does not. The same policy is applied and tested in the about crate under the ERR-7 / TASK-0977 label — see `extensions-rust/about/src/units.rs:196-232` and its `crate_metadata_breadcrumb_debug_escapes_control_characters` test.

<!-- scan confidence: single verified call site, provider.rs:33 -->

**Why it matters**: log forging is the standard consequence of unescaped untrusted data in log records (OWASP A09). The blast radius here is small — one `warn` in a CLI — but the fix is one character (`%member` to `?member`), the policy is already written down in this file, and leaving the one non-conforming site makes the stated invariant false.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The member field in the fallback warn uses the ? (Debug) formatter so control characters and ANSI escapes are escaped
- [ ] #2 A test asserts the rendered breadcrumb for a member containing a newline and an ESC byte contains neither raw character
<!-- AC:END -->
