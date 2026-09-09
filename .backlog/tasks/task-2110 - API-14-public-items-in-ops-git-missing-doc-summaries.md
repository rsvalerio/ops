---
id: TASK-2110
title: 'API-14: public items in ops-git missing doc summaries'
status: To Do
assignee: []
created_date: '2026-09-08 06:53'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions/git/src/lib.rs
  - extensions/git/src/provider.rs
  - extensions/git/src/config.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: Public items in `ops-git` with no doc comment at all:

- `extensions/git/src/lib.rs:30` `pub const NAME`
- `extensions/git/src/lib.rs:31` `pub const DESCRIPTION`
- `extensions/git/src/lib.rs:32` `pub const SHORTNAME`
- `extensions/git/src/lib.rs:34` `pub struct GitExtension`
- `extensions/git/src/provider.rs:11` `pub const DATA_PROVIDER_NAME`
- `extensions/git/src/provider.rs:99` `pub struct GitInfoProvider`
- `extensions/git/src/provider.rs:16-22` every field of `pub struct GitInfo` (`host`, `owner`, `repo`, `remote_url`, `branch`) — the values' contracts (lowercased host, scheme-preserving URL, `None` on detached HEAD) live only in the `schema()` strings and in `remote.rs`, not on the fields consumers actually read
- `extensions/git/src/config.rs:68` `RedactedUrl::as_str` and `:73` `RedactedUrl::into_string` — the two escape hatches out of the redaction newtype, i.e. the members most worth a doc line

**Why it matters**: These are the crate's entire public surface. `GitInfo`'s fields are `#[non_exhaustive]`-stable API consumed by other extensions (`resolve_repository_with_git_fallback` is called from every language's `project_identity` provider), and the field invariants are exactly what the SEC/PATTERN work in this crate established. Undocumented accessors on a security newtype (`as_str`/`into_string`) invite exactly the "route the inner string somewhere unredacted" refactor the type exists to make visible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every public item listed above has a one-line doc summary
- [ ] #2 GitInfo field docs state the invariant each field carries (lowercased host, scheme-preserving normalized URL, None semantics)
- [ ] #3 RedactedUrl::as_str / into_string docs state that the returned value is userinfo-free and control-character-free, and that callers must not re-introduce raw URLs
<!-- AC:END -->
