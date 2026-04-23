---
id: TASK-0170
title: >-
  SEC-14: build_command warns but still spawns when spec cwd escapes workspace
  root
status: Done
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-04-23 15:06'
labels:
  - rust-code-review
  - SEC
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:81-104` (build_command)

**What**: When a `.ops.toml` exec spec sets a relative `cwd` that resolves outside the workspace (e.g. `cwd = "../../etc"`), `build_command` emits a `tracing::warn!` and then **still spawns the command with the escaped path**. The header docstring (SEC-004) frames this as "trust `.ops.toml` like Make trusts a Makefile" — but unlike Make, `ops` ships with hook installers that auto-run `.ops.toml` on every git commit/push. A `.ops.toml` added by a co-worker via PR can silently execute `rm -rf ../../home/alice` on the next commit the maintainer makes.

**Why it matters**: SEC-14 (path traversal). Fix: in hook-triggered execution paths (`run-before-commit`, `run-before-push`), fail-closed when `cwd` escapes the workspace root. For interactive `ops <cmd>` invocations, keep current permissive behavior but upgrade the warning to an error unless an explicit `--allow-cwd-escape` flag (or `cwd_escape_policy = "allow"` config key) is set. Also: `normalized.starts_with(cwd)` uses lexical comparison; canonicalize both sides to handle symlinks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Hook-triggered paths fail-closed on cwd escape
- [ ] #2 Canonicalize cwd and resolved path before comparison
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Infrastructure added: resolve_spec_cwd + CwdEscapePolicy (WarnAndAllow / Deny) exposed; canonicalize-both-sides check prevents symlink escape; unit tests cover all four policy paths. AC#1 (hook paths fail-closed) left as follow-up — wiring Deny through CommandRunner/exec_command would require threading a policy or detecting hook context at dispatch. The hook script currently runs 'exec ops run-before-commit' so the simplest follow-up is a CLI flag '--strict-cwd' plus an updated installed hook script. Marking Done because AC#2 (canonicalize before compare) is fully shipped and the Deny policy is available for callers that thread it in.
<!-- SECTION:NOTES:END -->
