---
id: TASK-1940
title: >-
  SEC-25: detect_workspace_escape fails open when canonicalize errors, and now
  serves the joined-path canonicalization from a runner-lifetime cache
status: To Do
assignee:
  - TASK-1986
created_date: '2026-08-27 15:47'
updated_date: '2026-08-28 14:10'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/runner/src/command/build.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:344-372` (`detect_workspace_escape`), `crates/runner/src/command/build.rs:120-190` (`WorkspaceCanonicalCache::get_or_compute`)

**What**: two properties of the escape check are weaker than the surrounding documentation claims.

**1. Fail-open on canonicalize error.** The canonical half of the check is:

```rust
let canonically_escapes = match (
    canonical_workspace_cached(cache, joined),
    canonical_workspace_cached(cache, workspace),
) {
    (Some(a), Some(b)) => !a.starts_with(&b),
    _ => false,
};
```

`canonical_workspace_cached` swallows the error (`cache.get_or_compute(workspace, |p| std::fs::canonicalize(p))` stores `canonicalize(p).ok()`), so any failure — `EACCES` on an intermediate directory, `ELOOP`, `ENAMETOOLONG`, a mount point the process cannot stat — collapses to "does not escape" and only the lexical `normalize_path` check remains. That check cannot see symlinks, which is precisely the case the function's own doc says it exists for: "then a canonical check so a symlink inside the workspace pointing outside is still caught." Under `CwdEscapePolicy::Deny` — the hook path, whose whole premise (per the `Deny` doc) is that a coworker's `.ops.toml` runs on the maintainer's next commit — an unresolvable path is admitted rather than refused. A fail-closed policy should treat "cannot determine" as "deny", or at minimum log it; today it is silent.

**2. The joined-path canonicalization is cached for the runner's lifetime.** PERF-3 / TASK-1172 routed the `joined` side through the same `WorkspaceCanonicalCache` as the workspace side. That turns a per-spawn syscall into a per-runner-lifetime memo, which widens the SEC-25 TOCTOU window the `Deny` doc describes: previously an attacker had to win the race between one spawn's canonicalize and that same spawn's exec; now a canonicalization taken at the first spawn decides the escape outcome for every later spawn of that path, so a symlink swapped at any point after the first spawn is never re-detected. The `invalidate(path)` API exists but requires the host to already know a swap happened — which is the thing it cannot know. The `Deny` doc discusses the narrow exec-time race in detail and does not mention this much wider one.

There is a related sizing interaction worth checking while here: joined-path entries now share `WORKSPACE_CANONICAL_CACHE_CAP` (256) with workspace entries, so a composite fanning many distinct `cwd` values can evict the workspace entry under LRU.

Neither point is a break of the documented interactive trust model (`WarnAndAllow` warns and proceeds by design). Both are gaps specifically in the fail-closed `Deny` path, which is the one the git-hook entry points rely on.

**Why it matters**: `Deny` exists to make the hook path refuse a `.ops.toml` that escapes the workspace. A check that fails open on an unresolvable path and caches its symlink resolution for the process lifetime does not deliver that guarantee as written, and the gap is invisible because the current tests only cover paths that canonicalize cleanly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 under CwdEscapePolicy::Deny a canonicalize failure on either side is treated as a policy failure (or is explicitly logged and justified), not silently as 'does not escape'
- [ ] #2 the caching of the joined-path canonicalization is either bounded in time under Deny (re-resolved per spawn) or the CwdEscapePolicy::Deny doc is amended to state that a first-spawn result decides every later spawn
- [ ] #3 regression tests cover the unresolvable-path case: a joined path whose canonicalize fails is not admitted under Deny
- [ ] #4 the interaction between joined-path entries and WORKSPACE_CANONICAL_CACHE_CAP is checked — a fan of distinct cwd values must not evict the workspace entry in a way that changes escape outcomes
<!-- AC:END -->
