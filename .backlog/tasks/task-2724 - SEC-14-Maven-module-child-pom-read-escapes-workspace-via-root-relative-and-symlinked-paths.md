---
id: TASK-2724
title: 'SEC-14: Maven module child-pom read escapes workspace via root-relative and symlinked paths'
status: To Do
assignee: []
created_date: '2026-11-18 18:34'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2848'
modified_files:
  - extensions-java/about/src/maven/mod.rs
  - extensions-java/about/src/maven/pom.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-14:extensions-java/about/src/maven/mod.rs:collect_units'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/mod.rs:89` (`collect_units`), `extensions-java/about/src/maven/pom.rs:150` (`parse_pom_xml`)

**What**: `<module>` entries are untrusted manifest text. The in-tree guard in `collect_units` rejects only `Path::is_absolute()` and `Component::ParentDir`. SEC-14 also requires rejecting `Component::RootDir` and `Component::Prefix`: on Windows `\evil` and `C:evil` are not `is_absolute()`, yet `cwd.join(..)` replaces the base with them. The sibling Go collector (`extensions-go/about/src/modules.rs`) already matches `RootDir | Prefix(_)`.

Second, `parse_pom_xml(&cwd.join(&module))` passes the **child directory** as the containment root to `manifest_io::read_optional_text(&path, project_root, ..)`. If `<module>core</module>` names an in-tree symlink `core -> /elsewhere`, both the manifest and the "root" canonicalize under `/elsewhere`, so the containment check passes and a pom.xml outside the workspace is read into the unit name/version.

**Why it matters**: The crate's own comment says hostile `<module>` paths must never reach `join` + read; this is the same policy enforced inconsistently. Impact is limited (only name/artifactId/version of the out-of-tree file surface on the units page), so severity is low.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The in-tree check rejects RootDir and Prefix components in addition to absolute and ParentDir
- [ ] #2 Child pom reads are contained against the workspace root (cwd), not the child directory, so a symlinked module directory pointing outside the workspace is not read
- [ ] #3 Tests cover a symlinked module directory (unix) and the unit is still emitted for count parity
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.08 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
