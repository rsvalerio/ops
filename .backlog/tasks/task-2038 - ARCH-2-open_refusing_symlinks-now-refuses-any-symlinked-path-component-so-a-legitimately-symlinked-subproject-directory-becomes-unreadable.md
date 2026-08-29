---
id: TASK-2038
title: >-
  ARCH-2: open_refusing_symlinks now refuses any symlinked path component, so a
  legitimately symlinked subproject directory becomes unreadable
status: To Do
assignee:
  - TASK-2041
created_date: '2026-08-29 00:35'
updated_date: '2026-08-29 11:35'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - crates/core/src/text.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs` (`open_refusing_symlinks` / `unix_open::open_regular_no_symlink`)

**What**: TASK-1810 replaced the single `O_NOFOLLOW` open with a component-by-component `openat` walk, so a symlink at **any** component of the path is refused — not just the final one. That is the intended security boundary, but it is also a behaviour change for non-adversarial repos: a workspace that legitimately reaches a manifest through a symlinked directory (a monorepo that symlinks a shared subproject, a Gradle `settings.gradle` naming a symlinked include, a `Cargo.toml` `members` entry pointing at a symlinked crate) now gets `InvalidInput: refusing to follow symlink` instead of the manifest contents.

In practice the exposure is small: `std::env::current_dir()` returns a fully resolved path on Unix, so paths `ops` builds from its own cwd carry no symlinked ancestors. The reachable cases are paths joined from repo-supplied strings (`extensions-rust/about/src/units.rs`, `extensions-java/about/src/gradle/mod.rs`) and any embedder that hands `ops` an unresolved workspace root.

**Why it matters**: the failure is silent-ish — the about card drops a field or reports a wrong stack rather than erroring — so a user with a symlinked subproject sees degraded output with no explanation. The question to settle is whether the primitive should offer a root-anchored variant (`openat`-walk relative to a workspace-root fd, allowing symlinks in the operator-controlled prefix above the root) so the strict boundary applies only to the attacker-influenced suffix, and whether the refusal should surface a user-facing breadcrumb.

**Origin**: discovered during TASK-1984 while fixing TASK-1810.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A decision is recorded on whether open_refusing_symlinks gains a root-anchored variant that permits symlinks above a given workspace root while still refusing them beneath it
- [ ] #2 A refused symlink at an intermediate component leaves a tracing breadcrumb at the manifest-reading call sites, so a degraded about card is explainable
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Second observed symptom (code-review run 20260828, part 3): the same root cause also breaks `.ops.toml` loading, not just the about-card manifest readers. Path: `config::load_config_at` -> `read_config_file` -> `read_capped_toml_file` -> `read_capped_toml_file_with_policy(.., SymlinkPolicy::Refuse)` -> `text::open_refusing_symlinks` -> `unix_open::open_regular_no_symlink`, which applies O_NOFOLLOW per component. So a workspace root reached through a symlinked ancestor (an embedder or caller that passes an unresolved root; cwd-derived paths are still resolved by `current_dir`) makes `load_config_at` return InvalidInput 'refusing to follow symlink' from the local-.ops.toml layer instead of loading the file. Because `load_config_at` propagates with `?` at that point, the .ops.d and env layers never run either, so the whole config load fails rather than degrading. This is louder than the about-card symptom but has the same fix decision: whether the primitive gains a root-anchored variant that permits symlinks in the prefix above the workspace root. Not fixed in that run - deliberately deferred here so the trust boundary is decided once.
<!-- SECTION:NOTES:END -->
