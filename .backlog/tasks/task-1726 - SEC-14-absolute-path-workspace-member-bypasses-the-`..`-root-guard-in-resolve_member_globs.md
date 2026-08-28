---
id: TASK-1726
title: >-
  SEC-14: absolute-path workspace member bypasses the `..` root guard in
  resolve_member_globs
status: To Do
assignee:
  - TASK-2003
created_date: '2026-08-27 11:11'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/about/src/workspace.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:55-61`

**What**: The traversal guard rejects a member only when its path contains a `Component::ParentDir`:

```rust
if Path::new(member)
    .components()
    .any(|c| matches!(c, Component::ParentDir))
{
    tracing::warn!(member, "workspace member contains `..` traversal; skipping");
    continue;
}
```

An **absolute** member value has no `ParentDir` component — `/etc/foo` decomposes to `RootDir, Normal, Normal` — so it passes the check. `Path::join` then discards the base entirely: `root.join("/etc/foo")` is `/etc/foo`, not `<root>/etc/foo`. The same holds on the glob branch (`root.join(prefix)` at line 85) and on the non-glob branch (`root.join(member)` at line 159).

Concretely, `members = ["/home/user/.ssh"]` with `marker = "package.json"` resolves `try_read_manifest("/home/user/.ssh", "package.json")`, and `members = ["/etc/*"]` enumerates `/etc` and reads any `package.json` found under it. The resolved `(path, contents)` tuple then flows to the callers, which surface manifest contents and the absolute path in rendered `about` output (the `strip_prefix(root)` miss falls back to the absolute path by design, per `recover_relative_path`).

Impact is bounded because `[workspace].members` is operator-authored config, and `read_optional_text` caps reads at 4 MiB — but that is exactly the argument the `..` guard already rejected. The guard's stated purpose (line 50-54: "`root.join(member)` is otherwise the only surface where a `../sibling` entry escapes the workspace root") is not achieved while the absolute form is open, and an absolute member is the *simpler* escape of the two.

**Why it matters**: the containment invariant the guard exists to establish is not actually established, so a reviewer reading the guard concludes members cannot escape `root` when they can. A checked-in `package.json` from an untrusted repository can direct reads outside the checkout and echo their contents into the rendered card grid.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 resolve_member_globs rejects member values that are absolute (Component::RootDir / Prefix) with the same warn-and-skip treatment as ParentDir
- [ ] #2 A test asserts an absolute member (e.g. an OS-absolute path to a tempdir sibling holding a valid marker file) resolves to nothing while a valid relative sibling member in the same call still resolves
- [ ] #3 The guard's doc comment states the full containment invariant it now enforces (no ParentDir and no absolute prefix)
<!-- AC:END -->
