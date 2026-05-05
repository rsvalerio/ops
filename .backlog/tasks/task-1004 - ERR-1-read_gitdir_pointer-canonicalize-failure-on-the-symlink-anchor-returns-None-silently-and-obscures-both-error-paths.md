---
id: TASK-1004
title: >-
  ERR-1: read_gitdir_pointer canonicalize failure on the symlink-anchor returns
  None silently and obscures both error paths
status: Done
assignee: []
created_date: '2026-05-04 22:03'
updated_date: '2026-05-05 00:58'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:103-118`

**What**: SEC-14's symlink-redirection guard canonicalizes the `MAX_GITDIR_PARENT_TRAVERSAL`-step ancestor (`anchor_raw`) and the joined gitdir target, then `.starts_with(&anchor)`-checks them. Both `canonicalize()` calls swallow their error via `.ok()?` and return `None`. The downstream walker then continues to the parent directory (line 45-46 in `find_git_dir`), so a transient canonicalize error (EACCES on a parent dir, intermittent NFS, ENOENT on a worktree mid-rebuild) is structurally indistinguishable from the security-rejection path that the comment at line 109-118 describes.

**Why it matters**:
- ERR-1 / observability: the security-rejection branch logs at `tracing::debug!` (lines 112-117) so the operator can see SEC-14 fired. The two earlier `.ok()?` sites (anchor canonicalize, target canonicalize) emit *no* log. From the operator's POV, "git dir not found" can come from (a) the legitimate "no .git anywhere up the tree" case, (b) a SEC-14 escape rejection, or (c) a canonicalize syscall error — but only one of those three leaves a breadcrumb. The pre-commit hook then runs as if the repo doesn't exist, which surfaces as the (often-mysterious) "ops command did nothing" outcome.
- This is the exact `.ok()?` pattern that ERR-1 / READ-5 elsewhere in the codebase has been hardened away from (TASK-0517, TASK-0942, TASK-0935): every silent `?` over an IO Result that returns `None` now logs at `tracing::debug!` minimum so a future operator chasing the symptom has a thread to pull.

**Recommended fix**: replace each `.ok()?` with a match that logs at debug (different message per branch) before returning None, mirroring the policy already adopted at line 79-83 (`failed to read .git pointer file`) and 112-117 (`gitdir pointer escapes worktree-root anchor`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both canonicalize sites at extensions/hook-common/src/git.rs:107 and :108 emit a tracing::debug! breadcrumb before returning None on error.
- [ ] #2 Each site has a distinct log message so an operator can tell the anchor-failure case from the target-failure case.
- [ ] #3 Existing SEC-14 escape-rejection log message is unchanged.
<!-- AC:END -->
