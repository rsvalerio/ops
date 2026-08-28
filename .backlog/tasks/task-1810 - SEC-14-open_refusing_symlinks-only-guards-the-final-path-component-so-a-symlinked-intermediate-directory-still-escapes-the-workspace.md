---
id: TASK-1810
title: >-
  SEC-14: open_refusing_symlinks only guards the final path component, so a
  symlinked intermediate directory still escapes the workspace
status: To Do
assignee:
  - TASK-1984
created_date: '2026-08-27 11:31'
updated_date: '2026-08-28 14:09'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/text.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:152-181` (`open_refusing_symlinks`)

**What**: `open_refusing_symlinks` is the crate's single symlink-refusal primitive — `read_capped_to_string`, `for_each_trimmed_line`, and `config::loader::mod.rs:103` all route through it. On Unix it opens with `libc::O_NOFOLLOW`, which by definition applies **only to the last component of the path**. Every intermediate directory is still resolved through symlinks by the kernel.

The module doc states the threat model as *"an adversarial repo can plant `package.json -> /etc/passwd` (or `.ops.toml -> /etc/shadow`) and leak privileged file contents through diagnostics"* and the docstring does honestly narrow the guarantee to "the final path component". But callers consume it as *the* symlink defence, and the narrowed form does not cover the reachable attack:

- `extensions-rust/about/src/units.rs:186-200` calls `ops_core::text::read_capped_to_string(crate_toml_path)` for every workspace member listed in the root `Cargo.toml`. The member string is attacker-controlled in a hostile repo.
- `resolve_crate_display_name` guards that with `member_path_is_workspace_safe` (SEC-14 / TASK-1246), which rejects **absolute paths and `..`** — but not a plain relative name.
- So a repo shipping `members = ["evil"]` plus a symlink `evil -> /etc` yields the open of `evil/Cargo.toml`. `O_NOFOLLOW` inspects only `Cargo.toml`; `evil` is followed, and `/etc/Cargo.toml` (or any nested path an attacker can name) is read and its contents surfaced through about-card output and `tracing` breadcrumbs.
- The same shape applies to the Gradle line scanners in `extensions-java/about/src/gradle/mod.rs`, which join subproject paths read from `settings.gradle` before calling `for_each_trimmed_line`.

**Why it matters**: This is the exact information-disclosure primitive TASK-1442 / TASK-1461 / TASK-1468 added `O_NOFOLLOW` to close, left reachable one directory level up. `ops` is explicitly designed to be run inside third-party repositories, so "the repo is hostile" is the stated threat model, not a hypothetical one. Severity is reduced one level from the SEC-14 baseline because the docstring does accurately scope the guarantee (per the classification note on documented justifications) — but the scoping is a caveat in a doc comment, not an enforced boundary, and no caller currently compensates for it.

Fix direction (any one is sufficient): resolve the path component-by-component with `openat(…, O_NOFOLLOW | O_DIRECTORY)` from a workspace-root fd; use Linux `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS` where available; or canonicalize the path and assert the result is still under the workspace root before opening. Whichever is chosen, the primitive should state the boundary it enforces rather than the syscall flag it sets.

<!-- scan confidence: verified — O_NOFOLLOW semantics are last-component-only, and the units.rs / gradle callers above were read to confirm reachability -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 open_refusing_symlinks (or a replacement primitive) refuses to open a path whose resolution traverses a symlink at ANY component, not only the final one
- [ ] #2 A regression test creates dir/link -> /tmp/outside plus a target file, then asserts that reading <root>/link/<manifest> is refused with the stable InvalidInput surface
- [ ] #3 The rustdoc states the boundary the primitive enforces (e.g. 'the opened file is beneath <root> and no component is a symlink') rather than naming the O_NOFOLLOW flag, and the non-Unix fallback documents the residual TOCTOU gap
<!-- AC:END -->
