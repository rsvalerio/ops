---
id: TASK-1488
title: >-
  READ-1: command_registry_from_iter_drains_duplicate_audit_trail doc comment
  merges SEC-21 and DUP-3 paragraphs
status: Done
assignee:
  - TASK-1579
created_date: '2026-05-18 06:28'
updated_date: '2026-05-19 17:04'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/tests.rs:526-541` (the rustdoc block immediately above `command_registry_from_iter_drains_duplicate_audit_trail`)

**What**: The doc comment on `command_registry_from_iter_drains_duplicate_audit_trail` opens with a SEC-21 / TASK-1226 paragraph about `DataRegistry::register` formatting via `?` (Debug), then ends mid-sentence (`"Pin the value-level escape directly,"`) and switches abruptly to a DUP-3 / TASK-1225 paragraph about `CommandRegistry::collect()` audit-trail draining. The SEC-21 narrative belongs on `provider_name_field_debug_escapes_control_characters` (which it currently lacks a leading paragraph for — that test only has the trailing `/// mirroring ...` fragment).

```rust
/// SEC-21 / TASK-1226: `DataRegistry::register` formats the runtime-
/// generated `provider_name` field via the `?` (Debug) formatter so an
/// extension that builds a provider name from external data containing
/// newlines or ANSI sequences cannot forge log entries through the
/// duplicate-insert breadcrumb. Pin the value-level escape directly,
/// DUP-3 / TASK-1225: building a `CommandRegistry` via `collect()` /
/// `from_iter()` must NOT silently drop the duplicate-insert audit
/// trail. ...
#[test]
fn command_registry_from_iter_drains_duplicate_audit_trail() { ... }

/// mirroring `program_field_debug_escapes_control_characters`
/// (TASK-1127) and the broader workspace policy.
#[test]
fn provider_name_field_debug_escapes_control_characters() { ... }
```

Looks like an accidental merge / paste — the SEC-21 paragraph drifted onto the wrong `#[test]`, leaving the SEC-21 test with a stray trailing fragment.

**Why it matters**: READ-1 — confusing, factually misleading documentation. Future maintainers reading the DUP-3 test will wonder why the doc opens with SEC-21 formatter behaviour that is unrelated to the test body, and the SEC-21 test now lacks a coherent leading description that explains why it exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move the SEC-21 / TASK-1226 paragraph to be the leading doc block on provider_name_field_debug_escapes_control_characters, joining the existing "mirroring program_field_debug_escapes_control_characters" trailing fragment into one coherent paragraph
- [ ] #2 Leave the DUP-3 / TASK-1225 paragraph as the standalone doc block on command_registry_from_iter_drains_duplicate_audit_trail, removing the spliced SEC-21 prefix
<!-- AC:END -->
