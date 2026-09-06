---
id: TASK-1890
title: >-
  SEC-14: an absolute gitdir: pointer is returned unvalidated, skipping the
  containment anchor every relative pointer must pass
status: Done
assignee:
  - TASK-2008
created_date: '2026-08-27 15:34'
updated_date: '2026-08-28 23:03'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/hook-common/src/git.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:118-177` (`read_gitdir_pointer`)

**What**: The pointer resolver splits on absoluteness and only guards one branch:

```rust
let target = Path::new(rest.trim());
if target.is_absolute() {
    return Some(target.to_path_buf());   // <- no cap, no anchor, no containment
}
if max_parent_escape(target) > MAX_GITDIR_PARENT_TRAVERSAL { return None; }
... // canonicalize, then reject anything outside the two-levels-up anchor
```

A *relative* pointer must survive two checks (the textual `..` cap and the symlink-aware canonical containment against the anchor added by TASK-0788). An *absolute* pointer survives zero: `gitdir: /anywhere/on/disk/.git` is returned as-is, canonicalised by `probe_git_entry`, and handed to `install_hook`, which then writes an executable script into `<that>/hooks/<hook>`.

The only thing standing between a planted pointer and the write is `paths.rs::is_accepted_git_dir` — the target must be named `.git` (or be `<x>/.git/worktrees/<name>`) and contain a `HEAD` regular file. That check exists to stop the *filename heuristic* being the sole gate; it is not a containment boundary, and any real repository on the machine satisfies it.

Reachability: `find_git_dir` walks up from the cwd and probes each ancestor's `.git`. A `.git` **file** planted anywhere in that chain — an unpacked archive, a generated or vendored tree, a scratch directory a tool wrote — redirects the whole install. The doc comment on `MAX_GITDIR_PARENT_TRAVERSAL` records the assumption behind the asymmetry ("Real worktree pointers either use absolute paths or step up at most one or two directories"), i.e. absolute pointers are trusted because git writes them — but the parser cannot tell a pointer git wrote from one someone else wrote, which is the same reasoning that motivated the strict-shape parse in TASK-1245 immediately above.

**Why it matters**: `install_hook` is a write primitive that produces an *executable file git runs automatically*. The relative branch is defended in depth precisely because that is the payoff; the absolute branch reaches the same write with no containment at all. Realistic impact is bounded by filesystem permissions (the redirect can only land somewhere the invoking user could already write), so this is defence-in-depth rather than a privilege boundary — but it is the asymmetry that makes it a finding: two spellings of the same input get very different scrutiny. A cheap tightening is to canonicalise the absolute target and require it to sit under the walk's starting root (or the repository the walk began in), and to log the rejection at `debug!` like the other refusal paths do.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An absolute gitdir: target is canonicalised and validated before being returned, rather than returned verbatim
- [x] #2 The validation states an explicit containment rule for absolute targets (documented in the function doc alongside the existing MAX_GITDIR_PARENT_TRAVERSAL rationale) and rejects targets outside it with a tracing::debug! breadcrumb, matching the relative-pointer refusal paths
- [x] #3 A test plants a .git pointer file containing an absolute path to a HEAD-bearing directory outside the worktree and asserts find_git_dir refuses it
- [x] #4 A test asserts a legitimate absolute worktree pointer (the shape git itself writes) still resolves
<!-- AC:END -->
