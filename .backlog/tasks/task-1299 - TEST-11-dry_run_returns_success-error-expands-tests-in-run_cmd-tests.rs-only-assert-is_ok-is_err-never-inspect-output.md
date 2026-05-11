---
id: TASK-1299
title: >-
  TEST-11: dry_run_returns_success/error/expands tests in run_cmd/tests.rs only
  assert is_ok/is_err, never inspect output
status: To Do
assignee:
  - TASK-1305
created_date: '2026-05-11 16:20'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`crates/cli/src/run_cmd/tests.rs:368-388\`

**What**: Three tests in \`run_command_dry_run_tests\` call the \`run_command_dry_run\` variant that writes to the real process stdout (not the \`_to(... &mut writer)\` variant) and then only assert on the \`Result\`:

- \`dry_run_returns_success_for_known_command\` — asserts \`result.is_ok()\` and \`result.unwrap() == ExitCode::SUCCESS\`; never inspects what was printed.
- \`dry_run_returns_error_for_unknown_command\` — asserts \`result.is_err()\`; never inspects the error message.
- \`dry_run_expands_composite_commands\` — asserts \`result.is_ok()\`; never inspects whether the composite was actually expanded into its leaves.

The neighbouring \`dry_run_shows_program_and_args\`, \`dry_run_redacts_sensitive_env_vars\`, etc. already use \`run_command_dry_run_to\` with a \`Vec<u8>\` writer and assert against \`String::from_utf8(buf)\`, so the infrastructure for an actual content assertion is right there. Tests that only assert \`is_ok\` here are weak — they verify the function compiled and returned a Result, nothing about the dry-run preview the function exists to produce.

\`dry_run_expands_composite_commands\` is the load-bearing one: \"composite was expanded\" is the entire test name, but no assertion checks that the rendered output contains \`Resolved to 2 step(s)\` or \`[1] build\` (those exist verbatim in the neighbouring \`dry_run_composite_shows_all_steps\` test, which uses the \`_to\` variant).

**Why it matters**: TEST-11 says \"Assert specific values, not just is_ok / is_some\". A mutation that, say, deleted the entire body of \`run_command_dry_run_to\` after the leaf-id resolution would still leave \`dry_run_returns_success_for_known_command\` passing, defeating its purpose as a regression detector for the dry-run preview path. The fix is to route the three tests through \`run_command_dry_run_to(&runner, name, &mut buf)\` and assert against the captured bytes — the exact pattern the rest of the module already uses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 dry_run_returns_success_for_known_command captures stdout via run_command_dry_run_to and asserts the output contains the resolved program text
- [ ] #2 dry_run_returns_error_for_unknown_command asserts a message substring (e.g. 'nonexistent') in the returned error, not just is_err()
- [ ] #3 dry_run_expands_composite_commands captures stdout and asserts both leaf step labels appear in the rendered preview
<!-- AC:END -->
