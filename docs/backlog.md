# Backlog task management (`ops backlog`)

`ops backlog` manages the `.backlog/` markdown task tree natively — a
compatible subset of [Backlog.md](https://github.com/MrLesk/Backlog.md)
(the npm `backlog` CLI). It reads every file that tool has ever written to
this repository and writes files that tool can read back, so the two can
operate on the same tree during a migration.

Implementation: `crates/backlog` (parsing, store, rendering, handlers) wired
into the CLI at `crates/cli/src/backlog_cmd.rs`.

---

## Commands

```
ops backlog task create "<title>" [flags]     # prints `Created TASK-NNNN`
ops backlog task edit <taskId> [flags]        # prints `Updated TASK-NNNN`
ops backlog task list [flags]
ops backlog task view <taskId> [--plain|--json]
ops backlog search [query] [flags]
ops backlog cleanup [flags]
```

### `task create`

| Flag | Meaning |
|------|---------|
| `-d, --description <text>` | Description body (multi-line via shell heredoc) |
| `-a, --assignee <names>` | Assignees, comma-separated or repeatable |
| `-s, --status <status>` | Status; defaults to the config's `default_status` (`Triage` here) |
| `-l, --labels <labels>` | Labels, comma-separated or repeatable |
| `--priority <p>` | `critical` \| `high` \| `medium` \| `low` (default `low`) |
| `--ac <criterion>` | Acceptance criterion, repeatable (`#N` numbering is assigned on write) |
| `--modified-file <path>` | Repo-root-relative path the task touches, repeatable — the machine-readable twin of the `**File**:` line; wave triage reads it to compute file scope and merge order |
| `--plan <text>` | Implementation plan section |
| `--notes <text>` | Implementation notes section |
| `--depends-on <ids>` (`--dep`) | Dependency task ids, comma-separated or repeatable |
| `--plain` | Plain output (the only renderer) |

A fresh task gets `ordinal: 1000` (the value the backlog CLI assigns new
parent tasks) and no `updated_date`.

### `task edit`

| Flag | Meaning |
|------|---------|
| `-s, --status <status>` | New status (free-form string) |
| `-a, --assignee <names>` | **Replaces** the assignee list; `-a ""` clears it. This is how wave membership is recorded: a member's assignee is set to the wave parent's task id |
| `--add-label <labels>` | Add labels without replacing existing ones |
| `--append-notes <text>` | Append implementation notes, repeatable — inserted inside the existing SECTION:NOTES markers (blank-line separated) or as a new section |
| `--priority <p>` | New priority |
| `-t, --title <title>` | New title — **renames the file** to the new slug |
| `-d, --description <text>` | Replace the description |
| `--ac <criterion>` | Replace all acceptance criteria |
| `--check-ac N` / `--uncheck-ac N` | Set the 1-based criterion's checked state, repeatable |
| `--parent <taskId>` | Set `parent_task_id` — the structural member→parent link the npm CLI can only set at create time. Verified against the real binary: `list -p` and the view JSON resolve a plain `TASK-N` task carrying the field (no dotted subtask rename needed) |
| `--clear-parent` | Remove `parent_task_id` entirely (conflicts with `--parent`) |
| `--add-dep <ids>` | Append dependencies without replacing the list (dedup, comma-separated or repeatable) |
| `--remove-dep <ids>` | Remove individual dependencies — the surgical unlink the npm CLI lacks (it only has set-all and `--clear-deps`) |
| `--plain` | Plain output |

Every edit bumps `updated_date`.

### `task list`

| Flag | Meaning |
|------|---------|
| `-s, --status <statuses>` | Case-insensitive status filter, comma-separated or repeatable |
| `-a, --assignee <name>` | Keep tasks with any matching assignee (`task list -a code-review-wave` lists every wave parent) |
| `-l, --labels <labels>` | Keep tasks carrying **every** one of these labels (AND semantics, case-insensitive, comma-separated or repeatable) — the backlog CLI's `--labels` contract |
| `-p, --parent <taskId>` | Keep tasks whose `parent_task_id` matches — membership from the parent side |
| `--dependents <taskId>` | Keep tasks whose `dependencies:` include this id — the dependents-of-X reverse query the npm CLI lacks entirely |
| `--plain` | Plain text (the only non-JSON renderer) |
| `--json` | Versioned machine-readable JSON (mutually exclusive with `--plain`) |

`list` and `search` scan `.backlog/tasks/` **only** — matching the backlog
CLI, completed and archived tasks never appear in listings.

### `task view`

Resolves one id across `tasks/` → `completed/` → `archive/tasks/`, in that
precedence order (the tree contains four id collisions between `completed/`
and `archive/tasks/`; tasks/ wins). `--plain` and `--json` are mutually
exclusive.

### `search`

| Flag | Meaning |
|------|---------|
| `--modified-file <path>` | Keep tasks where any `modified_files` entry contains this substring |
| `--exclude-status <status>` | Drop tasks with this status, repeatable |
| `--plain` | Plain output |

Scoring is deterministic keyword containment (exact id match = 1.000;
otherwise per-token weights id 0.35 / title 0.30 / labels 0.15 /
description 0.10 / notes 0.05, averaged). Scores are intentionally **not**
compatible with the backlog CLI's fuzzy algorithm; the row shape is.

### `cleanup`

| Flag | Meaning |
|------|---------|
| `--older-than <days>` | Move terminal-status tasks older than this many days (default `30`) |
| `--dry-run` | Print the candidates without moving anything (no prompt) |

Moves terminal-status tasks (the **last** entry of the config's `statuses`,
matched case-insensitively) from `tasks/` to `completed/`, file unchanged.
A task's age reads `updated_date` with `created_date` as fallback; a task
with no parseable date is skipped, never moved on a technicality. After
listing the candidates it asks `Move N tasks to completed folder? [y/N]`
on stdin — `y`/`yes` proceeds, empty input or anything else cancels (No is
the default, like the backlog CLI's confirm). The age arrives as a flag
instead of the backlog CLI's interactive menu; `--dry-run` skips the prompt
entirely. Destinations are preflighted after the confirmation: a same-name
file already in `completed/` aborts the command naming both paths before
anything moves — all-or-nothing for collisions detected at preflight, the
tree holds real id collisions and a silent overwrite would destroy one of
them. A move is not itself atomic: the destination name is claimed
atomically with a no-replace link (`link(2)` refuses an existing name, so
a destination appearing between preflight and move is never overwritten),
then the source name is removed as a separate step — an interruption in
between leaves both names on one file, and a retry reports the leftover
destination as a collision to resolve by hand. A late collision, or any
other per-file failure, aborts the run with the earlier moves kept — there
is no rollback. Git staging stays with the caller.

---

## File format

One task per markdown file: `tasks/task-<NNNN> - <slug>.md`, ids zero-padded
to the configured width (4), slug from the title (runs of characters outside
`[A-Za-z0-9._-]` collapse to `-`, empty slug becomes `untitled`).

```markdown
---
id: TASK-2069
title: 'DUP-3: ...'            # single-quoted ('' escapes), folded >-, or bare
status: Done                    # free-form: Triage / To Do / In Progress / Done
assignee: []                    # ALWAYS a list (never a scalar)
created_date: '2026-08-29 18:21'  # oldest files carry :SS seconds
updated_date: '2026-08-31 17:38'  # absent in 4 pre-history files
labels:
  - code-review-rust
dependencies: []                # block list of TASK-NNNN when non-empty
modified_files:                 # only newer findings
  - crates/foo/src/lib.rs
priority: low                   # absent in 243 files
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
...
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ...
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
...
<!-- SECTION:NOTES:END -->
```

Rules the parser and writer honor:

- Only the **first** `---` pair bounds the frontmatter — YAML-looking text
  inside fenced code blocks in the body is inert.
- Unknown keys (`type`, `parent_task_id`, …) are preserved and re-emitted;
  `parent_task_id` renders right after `dependencies`, where the CLI puts it.
- Canonical write order: `id, title, status, assignee, created_date,
  updated_date, labels, dependencies, parent_task_id, modified_files,
  priority, ordinal`, then other extras in file order.
- Acceptance criteria are 100% `- [x] #N text` / `- [ ] #N text`; the `#N`
  prefix is regenerated from list position on every write.
- Hand-written unmarked sections (`## Closure`, `## Scope`, …) pass through
  edits byte-identical.
- 31 files in the tree are frontmatter-only — the body is optional.

## Config

`backlog.config.yml` at the workspace root (cwd-based discovery, no upward
walk; missing file → defaults). Honored keys: `statuses` (list-group order),
`default_status`, `backlog_directory`, `task_prefix`, `zero_padded_ids`.
Everything else (ports, git behaviour, board rendering) is skipped — those
features are not implemented.

## Output contracts

The code-review skills parse these shapes; they are pinned by golden tests
against backlog.md v1.51.0 and byte-verified against the live tree:

- `task view --plain` **first line**: `File: <absolute path>` — the
  run-wave skill extracts it with `sed -n '1s/^File: //p'`.
- `task list --plain`: status header (`Done:`) then
  `  [HIGH] TASK-1656 - title (ac: 5/5)` — the `(ac: …)` suffix is omitted
  when the task has no criteria; a `type` (e.g. `[enhancement]`) fills the
  bracket for unprioritized typed tasks.
- `task view --json` / `task list --json`: `{"schemaVersion": 1, "kind":
  "task-view"|"task-list", …}` envelopes with the CLI's field names and
  order (hand-rendered for exactly that reason). Dates normalize to
  ISO-8601 Z (`'2026-08-29 18:21'` → `"2026-08-29T18:21:00Z"`).
- `search --plain`: `Tasks:` header, rows
  `  TASK-1766 - title (Done) [LOW] [score 0.671]`.

## Scope — deliberately not implemented

Git integration (auto-commit, branch checks), the terminal board, the web
browser UI, the MCP server, milestones, docs/decisions, DoD defaults, the
init wizard, and interactive TUIs. Commits and locking stay with the caller
(the code-review skills own their merge lock and `chore(backlog)` commits).

## Compatibility testing

- `crates/backlog/tests/corpus.rs` — invariants over **copies of real task
  files** (the fenced-YAML trap, seconds dates, missing `updated_date`,
  frontmatter-only, bare titles, the `type: enhancement` file, both
  TASK-0059 duplicates): every fixture parses and render∘parse is a fixed
  point.
- Unit/integration goldens pin the output shapes above, including the
  actual `sed -n '1s/^File: //p'` extraction against a real `sed`.
