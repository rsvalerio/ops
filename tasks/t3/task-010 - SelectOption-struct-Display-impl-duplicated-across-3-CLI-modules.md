---
id: TASK-010
title: SelectOption struct + Display impl duplicated across 3 CLI modules
status: To Do
assignee: []
created_date: '2026-04-07 00:00:00'
updated_date: '2026-04-07 22:48'
labels:
  - rust-code-duplication
  - CD
  - DUP-2
  - DUP-3
  - DUP-6
  - low
  - effort-S
  - crate-cli
dependencies: []
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/cli/src/{run_before_commit_cmd.rs:9-18, extension_cmd.rs:299-308, theme_cmd.rs:154-165}`
**Anchor**: `struct CommandOption`, `struct ExtensionOption`, `struct ThemeOption`
**Impact**: Three near-identical structs used as `dialoguer` select options, each with a `name: String`, `description: String`, and a `Display` impl that formats as `"{name} - {description}"`. ThemeOption adds an `is_custom: bool` field.

**Notes**:
`CommandOption` (run_before_commit_cmd.rs:9-18):
```rust
struct CommandOption { name: String, description: String }
impl Display { write!(f, "{} — {}", self.name, self.description) }
```

`ExtensionOption` (extension_cmd.rs:299-308):
```rust
struct ExtensionOption { name: String, description: String }
impl Display { write!(f, "{} - {}", self.name, self.description) }
```

`ThemeOption` (theme_cmd.rs:154-165):
```rust
struct ThemeOption { name: String, description: String, is_custom: bool }
impl Display { write!(f, "{}{} - {}", self.name, marker, self.description) }
```

Minor inconsistency: `CommandOption` uses em-dash (`—`), others use hyphen (`-`).

Fix option: a shared generic struct in a common module, parameterized on extra fields or using a trait for the Display suffix. Alternatively, a single `SelectOption` struct with an optional `suffix: Option<String>` field. Severity is low because each struct is module-private and the duplication is small (10 lines each).
<!-- SECTION:DESCRIPTION:END -->
