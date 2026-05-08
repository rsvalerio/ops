---
id: TASK-1126
title: >-
  ARCH-9: default_workspace_cache process-global static duplicates per-runner
  workspace_cache; runner invalidate has no effect on spawn path
status: To Do
assignee:
  - TASK-1261
created_date: '2026-05-08 07:28'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`crates/runner/src/command/build.rs:188-196\` (and \`crates/runner/src/command/mod.rs:139-153\`)

**What**: After TASK-1063 folded the workspace canonicalize cache onto \`CommandRunner.workspace_cache\`, the spawn-time call chain (\`build_command_async\` -> \`spawn_blocking\` -> \`build_command_with\` -> \`detect_workspace_escape\` -> \`canonical_workspace_cached\`) still routes through the process-global \`default_workspace_cache()\` static, not the runner's instance. \`CommandRunner::invalidate_workspace_cache\` and \`clear_workspace_cache\` operate on the runner's cache, but the spawn path reads from the static — so the public invalidate API is a no-op against the cache that actually decides escape outcomes.

**Why it matters**: Two parallel sources of truth for the same invariant (canonicalize(workspace) result). The runner-scoped LRU bound and lifetime guarantees TASK-1063 introduced apply to a cache nothing on the hot path consults. The known gap is documented inline ("today's build_command_async call from exec.rs still reads the static default cache for compatibility; this field is the authoritative per-runner instance and the migration target") but no follow-up backlog task tracks the migration. Without it, AC #3 of TASK-1063 (post-symlink-swap re-canonicalize via invalidate) is unreachable from the public API for production callers; only the standalone WorkspaceCanonicalCache test exercises it.

**Notes**: Concretely, \`detect_workspace_escape\` and \`resolve_spec_cwd\` in build.rs both call \`canonical_workspace_cached(workspace)\` -> static. Migrating means threading \`Arc<WorkspaceCanonicalCache>\` through \`build_command_with\` (and \`build_command_async\`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 build_command_with / build_command_async route canonicalize lookups through the runner-scoped workspace_cache, not the static default_workspace_cache
- [ ] #2 CommandRunner::invalidate_workspace_cache observably affects subsequent spawn-time canonicalize results in tests
- [ ] #3 default_workspace_cache static is removed (or restricted to a clearly documented compatibility shim with no production caller)
<!-- AC:END -->
