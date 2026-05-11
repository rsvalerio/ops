---
id: TASK-1296
title: >-
  API-1: new_command validate_command_name does not reject names that shadow
  built-in subcommands
status: To Do
assignee:
  - TASK-1303
created_date: '2026-05-11 16:18'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: \`crates/cli/src/new_command_cmd.rs:62\`

**What**: \`validate_command_name\` rejects empty / whitespace / control / path-separator / leading-dash inputs, but does NOT reject names that collide with clap-registered built-in subcommands (\`init\`, \`theme\`, \`extension\`, \`about\`, \`deps\`, \`tools\`, \`plans\`, \`new-command\`, \`run-before-commit\`, \`run-before-push\`). A user who runs \`ops new-command\` and types e.g. \`init\` will see the config-defined command written to \`.ops.toml\` under \`[commands.init]\`, but \`ops init\` will always resolve to the clap-defined subcommand (which is matched before the External catch-all). The on-disk entry is silently unreachable.

**Why it matters**: API-12 (TASK-1272) is the sibling concern (TOML-key / clap-name validity). This one is the opposite axis: the input *is* a valid clap name but it shadows a built-in, so the persisted \`.ops.toml\` entry can never be invoked through the documented \`ops <name>\` surface. The user observes nothing wrong at write time and only discovers the failure when their command never runs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 validate_command_name rejects names that match any built-in CoreSubcommand name (init, theme, extension, about, deps, tools, plans, new-command, run-before-commit, run-before-push)
- [ ] #2 Rejection message names the colliding built-in so the user can pick a different name
- [ ] #3 Unit test asserts every CoreSubcommand variant name is rejected; uses the same iteration shape as stack_specific_commands so future additions are covered
<!-- AC:END -->
