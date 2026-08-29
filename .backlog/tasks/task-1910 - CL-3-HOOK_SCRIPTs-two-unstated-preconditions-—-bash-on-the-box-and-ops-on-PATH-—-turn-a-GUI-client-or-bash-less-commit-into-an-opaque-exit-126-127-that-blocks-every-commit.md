---
id: TASK-1910
title: >-
  CL-3: HOOK_SCRIPT's two unstated preconditions — bash on the box and ops on
  PATH — turn a GUI-client or bash-less commit into an opaque exit 126/127 that
  blocks every commit
status: Done
assignee:
  - TASK-2009
created_date: '2026-08-27 15:40'
updated_date: '2026-08-28 23:25'
labels:
  - code-review-rust
  - cognitive-load
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:44`

**What**:

```rust
const HOOK_SCRIPT: &str = "#!/usr/bin/env bash\nexec ops run-before-commit\n";
```

Two environmental preconditions are baked in, neither documented nor checked at install time:

1. **`bash` must exist and be on PATH.** The script body contains nothing bash-specific — it is a single `exec`. `#!/bin/sh` would run everywhere; `#!/usr/bin/env bash` fails on Alpine/busybox images, minimal container builds, and BSD/Nix setups where bash is not installed. `env` then exits **127**, git reports `hook failed`, and every commit is blocked with a message that names bash, not ops. The legacy hooks this crate is written to recognise and replace all use `#!/bin/sh` (see the fixtures at lines 135, 157).

2. **`ops` must be on the PATH *of the process that invokes git*.** Git hooks inherit the environment of the caller, and GUI clients (IDE VCS panes, GitHub Desktop, SourceTree, Fork) are launched from the desktop session, not the login shell — so `~/.cargo/bin` and `~/.local/bin` are routinely absent. The hook then dies with `ops: command not found`, exit **127**, and git aborts the commit. The user's only clue is a 127 in a dialog box. `install_hook` already knows where the running binary is (`std::env::current_exe()`), so an absolute path — or a `command -v ops || exit 0`-style guard with an explicit diagnostic — is available at install time.

Both failures are indistinguishable, from git's point of view, from "your checks failed": non-zero exit, commit refused. Because this is the pre-commit path, the blast radius is every commit on that machine until the user finds and deletes `.git/hooks/pre-commit` by hand.

**Why it matters**: CL-3 — preconditions must be explicit (types, asserts, guard clauses), not relied on as undocumented invariants. Here the invariants are ambient properties of a *future* process environment that the installing process cannot observe, and the failure mode is a hard block on the developer's primary workflow with an error that does not name the tool responsible. The bash requirement in particular buys nothing: it is a gratuitous portability constraint on a one-line script.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 HOOK_SCRIPT uses #!/bin/sh, or a comment states the specific bash feature that requires it
- [x] #2 The installed hook resolves ops robustly (absolute path from current_exe at install time, or a PATH fallback) rather than depending on the invoking process's PATH
- [x] #3 If ops cannot be found at hook run time, the hook emits a diagnostic naming ops and the reinstall command on stderr before exiting, instead of surfacing a bare 127
- [x] #4 A test asserts the shebang and the resolution strategy so a future edit to the one-line script cannot silently reintroduce either assumption
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`HOOK_SCRIPT` rewritten on the model TASK-1911 already landed for the sibling pre-push
hook, so the two crates stay in lockstep:

- `#!/bin/sh`, no bash anywhere in the body (AC#1).
- `command -v ops` probe before the `exec` (AC#2). The PATH probe, not
  `current_exe()`, is the strategy — it matches run-before-push, and it keeps a hook that
  survives the binary being moved or reinstalled elsewhere, which an absolute path baked in
  at install time does not.
- On a miss the script writes two stderr lines naming `ops`, the hook path
  (`.git/hooks/pre-commit`), the reinstall command and the `SKIP_OPS_RUN_BEFORE_COMMIT`
  bypass, then exits 1 rather than surfacing a bare 127 (AC#3).

Tests (AC#4): `hook_script_uses_posix_sh_shebang` (asserts the shebang *and* that the string
contains no "bash" at all), `hook_script_guards_missing_ops_binary`,
`hook_script_is_valid_posix_sh` (`sh -n`), and
`hook_script_reports_a_missing_ops_binary_by_name`, which actually runs the script under
`/bin/sh` with a PATH that excludes the developer's own ops and asserts exit 1 plus the
diagnostic contents.

Also updated `extensions/hook-common/src/fixtures.rs`: both synthetic `HookConfig`
fixtures used `#!/usr/bin/env bash` stand-in scripts, so no fixture models a bash
dependency neither hook crate ships any more.

Correction to the note above: the `extensions/hook-common/src/fixtures.rs` change was made,
then **reverted before the merge**. Integration verify caught that
`install::tests::classify_existing_hook_separates_partial_from_foreign` (landed by a sibling
wave in this same run) hardcodes `"#!/usr/bin/env bash\nexec ops run-before-com"` as a strict
prefix of `commit_config().hook_script`; changing the fixture shebang reclassified that
literal from `Partial` to `Foreign` and failed the test. Rather than rewrite another wave's
test under a held merge lock, the discretionary fixture cleanup was dropped and filed as
TASK-2036 (Triage), which records the prefix coupling that has to be fixed first. None of
this task's own acceptance criteria depended on it.
<!-- SECTION:NOTES:END -->
