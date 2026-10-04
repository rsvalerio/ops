---
id: TASK-2360
title: 'ERR-14: package.json deserialisation errors do not name the failing field'
status: Triage
assignee: []
created_date: '2026-10-04 14:10'
labels:
  - code-review-rust
  - err
dependencies: []
modified_files:
  - extensions-node/about/src/package_json.rs
  - extensions-node/about/src/units.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-14:extensions-node/about/src/package_json.rs:parse_package_json'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:92` (`parse_package_json`), `extensions-node/about/src/units.rs:~140` (`workspace_member_globs`)

**What**: `serde_json::from_str::<RawPackage>` / `<RawRoot>` run on a human-edited manifest. `RawPackage` uses several `#[serde(untagged)]` enums (`LicenseField`, `RepositoryField`, `PersonField`) and `RawRoot` uses `WorkspacesField`; a wrong-typed field there fails with serde's generic "data did not match any variant of untagged enum ..." plus line/column, never the field name. The `warn_parse_failure` record therefore cannot tell the operator which field (`license`, `repository`, `workspaces`) to fix.

**Why it matters**: The whole manifest falls back to defaults on one bad field, and the warn is the only diagnostic. Wrapping in `serde_path_to_error::deserialize` (or a per-field lenient deserialiser) would name the field. Diagnostics only; SEC-11 limits are unaffected.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Parse-failure warn records include the JSON field path of the failure
- [ ] #2 A test with a wrong-typed license/repository/workspaces field asserts the path appears in the warn
<!-- AC:END -->
