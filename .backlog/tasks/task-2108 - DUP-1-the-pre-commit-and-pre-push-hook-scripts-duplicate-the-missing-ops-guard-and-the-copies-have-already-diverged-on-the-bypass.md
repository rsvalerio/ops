---
id: TASK-2108
title: 'DUP-1: the pre-commit and pre-push hook scripts duplicate the missing-ops guard, and the copies have already diverged on the bypass'
status: Done
assignee: []
created_date: '2026-09-08 06:53'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2234'
modified_files:
  - extensions/run-before-commit/src/lib.rs
  - extensions/run-before-push/src/lib.rs
  - extensions/hook-common/src/lib.rs
priority: high
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:50` (`HOOK_SCRIPT`), `extensions/run-before-push/src/lib.rs:46` (`HOOK_SCRIPT`)

**What**: Both hook scripts embed the same hand-written `command -v ops` preflight — same shape, same two `echo ... >&2` diagnostic lines, same `exit 1` — differing only in the hook name and the skip variable spelled into the message. The two copies have already drifted:

- run-before-commit (lines 56-63) evaluates the bypass **before** the probe:
  `case "${SKIP_OPS_RUN_BEFORE_COMMIT:-}" in 1 | [Tt][Rr][Uu][Ee] | ... ) exit 0 ;; esac`
  then probes for `ops`.
- run-before-push (lines 47-51) has **no** bypass arm at all, yet its own diagnostic tells the user to "bypass with SKIP_OPS_RUN_BEFORE_PUSH=1". With `ops` off PATH the hook exits 1 before anything reads that variable, so the advertised escape hatch does not work in the one situation the message advertises it for.

The commit crate documents this exact reasoning on its own copy ("the bypass is honoured before the probe below: that probe's own diagnostic advertises this variable") and pins it with `hook_script_honours_the_bypass_when_ops_is_missing`; none of that reached the sibling, because the guard is copy-pasted source text rather than shared.

**Why it matters**: A developer whose PATH lacks `~/.cargo/bin` (GUI git clients, IDE VCS panes) is blocked from pushing, and the only advice the tool gives them is advice that does not work — the remaining escape is `git push --no-verify` or deleting the hook by hand. The duplication is also the mechanism: any future fix to the guard has to be made twice, and this divergence shows that does not happen. `ops_hook_common` already owns `HookConfig` (name, hook_filename, skip_env_var, ...), so the guard is expressible once as a shared prologue parameterised by the config the macro already receives.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The bypass-then-probe prologue exists in exactly one place (a shared const or a fragment generated from HookConfig in ops-hook-common), not once per hook crate
- [ ] #2 The pre-push hook honours SKIP_OPS_RUN_BEFORE_PUSH (1/true/yes/on, case-insensitive) and exits 0 before the missing-ops probe, matching the pre-commit behaviour
- [ ] #3 A test drives the pre-push script with ops off PATH and each documented truthy token and asserts exit 0, mirroring hook_script_honours_the_bypass_when_ops_is_missing
- [ ] #4 Both scripts still pass sh -n and still name ops, the hook path, and their own skip variable on stderr when ops is missing
<!-- AC:END -->
