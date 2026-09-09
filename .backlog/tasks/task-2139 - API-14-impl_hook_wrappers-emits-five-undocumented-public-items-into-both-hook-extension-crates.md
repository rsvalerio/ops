---
id: TASK-2139
title: 'API-14: impl_hook_wrappers! emits five undocumented public items into both hook extension crates'
status: To Do
assignee: []
created_date: '2026-09-08 06:56'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions/hook-common/src/lib.rs
priority: low
ordinal: 55000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:104` (`impl_hook_wrappers!`)

**What**: the macro body attaches a doc comment to exactly one of the six
public items it generates. The other five arrive in the expanding crate with
no doc summary at all:

- `pub const HOOK_CONFIG: HookConfig`
- `pub fn hook_config() -> HookConfig`
- `pub fn should_skip() -> bool`
- `pub fn find_git_dir(from: &Path) -> Option<PathBuf>`
- `pub fn install_hook(git_dir: &Path, w: &mut dyn Write) -> anyhow::Result<PathBuf>`

Only `ensure_config_command` carries `///` text. Two of the undocumented items
return `anyhow::Result` (`install_hook`) or already have a documented
`# Errors` contract upstream that is not forwarded, and none of them link back
to the `ops_hook_common` function they delegate to — so `cargo doc` for
`ops-run-before-commit` and `ops-run-before-push` renders a public surface
with blank summaries and no `# Errors` section on the fallible one.

The macro is the right place to fix it: whatever is written there is inherited
by both wrapper crates and by any future hook crate, which is the reason the
macro exists.

**Why it matters**: these are the public entry points of two shipped extension
crates. Nothing catches the gap automatically — `missing_docs` is not enabled
in `[workspace.lints.rust]` and clippy's `missing_errors_doc` does not see
through macro expansion reliably — so it will not surface on its own. Also
worth resolving while editing: `HOOK_CONFIG` and `hook_config()` are two public
spellings of the same value (API-13), and if both are kept the docs should say
which one callers are meant to use.

Low severity: documentation only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every public item emitted by impl_hook_wrappers! carries a doc summary that links to the ops_hook_common function it wraps
- [ ] #2 The generated install_hook (and ensure_config_command) carry a '# Errors' section
- [ ] #3 The HOOK_CONFIG const vs hook_config() fn duplication is either removed or documented with a stated reason for keeping both
- [ ] #4 cargo doc for ops-run-before-commit and ops-run-before-push shows no blank-summary public items from the macro
<!-- AC:END -->
