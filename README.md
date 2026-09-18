# ops

An opinionated, batteries-included development CLI operator.

## Installation

### Homebrew (macOS and Linux)

```bash
brew install rsvalerio/tap/ops
```

### Local development

```bash
cargo install --path crates/cli
```

## Quick start

```bash
# Initialize config for your project (auto-detects stack)
ops init

# Run a command
ops build

# Run static checks (fmt, check, clippy, build)
ops verify

# Run tests and quality checks
ops qa

# Add a new command interactively
ops new-command "cargo fmt --check"
```

## Configuration

Create a `.ops.toml` file in your project root (or run `ops init`):

```toml
[output]
theme = "classic"        # "classic" (default) or "compact"
columns = 80             # line width for step lines
show_error_detail = true # show error details below failed steps

[commands.build]
program = "cargo"
args = ["build", "--all-targets"]

[commands.test]
program = "cargo"
args = ["test"]

[commands.verify]
commands = ["fmt", "check", "clippy", "build"]
parallel = true
fail_fast = true

[commands.qa]
commands = ["test", "deps"]
parallel = true
fail_fast = true
```

Config is merged in order (later overrides earlier): built-in defaults → global config (`~/.config/ops/config.toml`) → local `.ops.toml` → `.ops.d/*.toml` fragments (sorted by filename) → `OPS__*` environment variables. When run inside a project with a detected stack (e.g. Rust), `ops init` pre-fills stack-specific commands.

### Extending existing commands

To add steps to a command that already exists — typically a stack default like the Rust `verify` — without copying (and going stale on) its whole `commands` list, use an `[extend.<name>]` section:

```toml
[commands.coverage]
program = "cargo"
args = ["llvm-cov"]

[extend.verify]
commands = ["coverage"]   # appended to the end of verify's commands list
```

Exec commands extend the same way, with `args` instead of `commands`:

```toml
[extend.clippy]
args = ["--locked"]       # added to clippy's args, before any `--` separator
```

The extras are appended at load time. Rules:

- Composites (`commands = [...]`) extend with `commands`; exec commands extend with `args`. Using the wrong key for the target's kind, extending an undefined name, or an entry that sets neither key is a load error naming the target.
- Appended `args` land **before the target's first `--` separator** when one is present, otherwise at the end of the args. Cargo commands like the Rust `clippy ... -- -D warnings` pass everything after `--` to the wrapped tool, so inserting before it keeps `--locked` a cargo flag instead of silently turning it into a lint flag.
- A locally redefined command wins: `[extend.verify]` appends to *your* `[commands.verify]` if you defined one, otherwise to the stack default.
- Extends concatenate across config layers, so `.ops.d/*.toml` fragments stack on top of `.ops.toml` appends.
- Extending controls list order only, not execution order. Each appended command keeps the `exclusive` flag of its own definition. In a sequential group it runs after the earlier steps. In a parallel group (see below), an appended non-exclusive command joins the final stage and may run concurrently with the earlier non-exclusive steps. Mark it `exclusive = true` if it must not overlap them.

### Cloning existing commands

To define a command as a variant of an existing one — typically a stack default — without copying (and going stale on) its whole spec, use `clone`:

```toml
[commands.fuzz-clippy]
clone = "clippy"

[extend.fuzz-clippy]
args = ["--manifest-path", "fuzz/Cargo.toml"]
```

`fuzz-clippy` is a copy of the resolved Rust `clippy` default (program and args included), and `[extend.fuzz-clippy]` adds the fuzz-specific flag — so the variant tracks the default's flags as they evolve. Composites clone the same way (`clone = "verify"` copies the `commands` list). The extras are materialized at load time, so `ops --dry-run fuzz-clippy` shows the resolved program and args. Rules:

- The source resolves like an `[extend]` target: your `[commands]` entry if you defined one, otherwise the detected stack's default. Extension-registered commands cannot be cloned — they register after config load — and naming one is an unknown-source load error.
- Scalar fields beside `clone` (`help`, `category`, `aliases`, and for exec sources `env`, `cwd`, `timeout_secs`, `exclusive`) override the copy; fields left unset keep the source's value. Given maps and lists replace the copy (`env` replaces, it does not merge). `aliases` are the exception: they are never inherited — a clone with no `aliases` has none, because inheriting the source's would either collide at load or silently redirect the source's alias to the clone. `program`, `args` and `commands` beside `clone` are load errors — extra args go through `[extend.<name>]`.
- A clone copies the source **before** the source's own `[extend.<source>]` applies: extends stay per-name, so the clone never inherits them. `[extend.<clone>]` applies to the materialized copy.
- Unknown sources, clone cycles (including self-clones; non-cyclic clone-of-clone chains do resolve), cloning into an existing stack-default name, and exec-only fields beside a composite source are load errors naming the command and the source.

### Command groups and scheduling

A command with a `commands = [...]` list is a *group* (composite). Groups may
reference other groups, and `ops` expands the whole tree into a single flat plan
that is scheduled as one unit.

Because the plan is scheduled as one unit, the group you invoke decides how it
runs:

- **Sequential root:** every step runs one at a time, including the steps of a
  nested `parallel = true` group. Running a parallel group sequentially is always
  safe, so a hook group such as `run-before-commit = ["verify", ...]` keeps
  working when `verify` itself is parallel.
- **Parallel root:** a nested group must not declare `parallel = false`. Its
  steps would run concurrently despite the flag, so the config is rejected with
  an error naming both groups:

```toml
[commands.fixers]
commands = ["ruff", "black"]
parallel = false

[commands.verify]
commands = ["fixers", "pyright"]
parallel = true          # error: conflicts with fixers.parallel = false
```

```console
$ ops verify
error: conflicting `parallel` in the plan for `verify`: `verify` sets parallel = true,
but `fixers` sets parallel = false
```

To fix, set `verify.parallel = false`, or keep it parallel and mark the steps
that must not overlap `exclusive = true` (below). Every group in a plan must
also declare the same `fail_fast`.

Note that this applies *within* one plan. Naming several commands on one
invocation (`ops run verify qa`) expands each independently, so they may differ.

> Expressing "run these groups in order, but let the steps inside one group run
> together" is not supported today; it needs per-group scheduling boundaries.
> To keep a single step from overlapping the rest, use `exclusive` (below).

### Exclusive steps in a parallel group

Running every step of a parallel group at once is wrong for a step that
rewrites files the others read. Mark such an exec command `exclusive = true`.
The plan is then split into ordered stages at each exclusive step: the
exclusive step runs alone, and each run of consecutive non-exclusive steps
between them runs concurrently. Stages follow the order of `commands`.

```toml
[commands.fmt]
program = "cargo"
args = ["fmt", "--all"]
exclusive = true

[commands.verify]
commands = ["fmt", "clippy", "build", "doc"]
parallel = true
# runs: fmt → (clippy | build | doc)
```

The list order is the schedule. With `a` and `c` exclusive,
`["a", "b", "c", "d"]` runs `a → b → c → d`, while `["a", "c", "b", "d"]` runs
`a → c → (b | d)`. Under `fail_fast = true` a failing stage stops the plan and
later stages never start. `exclusive` has no effect in a sequential group or
under `--raw`, which always runs sequentially.

## Commands

### Stack-agnostic CLI (same on every stack)

| Command | Description |
|---------|-------------|
| `ops <name>` | Run a configured command or command group |
| `ops init` | Create `.ops.toml` (minimal by default; `--force` to overwrite; `--output`/`--themes`/`--commands` add those sections, with stack-detected commands under `--commands`) |
| `ops new-command` | Add a new command from a command line string |
| `ops import-makefile` | Import Makefile targets as `.ops.toml` commands (interactive picker) |
| `ops theme list\|select` | List or select output themes |
| `ops extension list\|show` | List compiled-in extensions |
| `ops about [setup\|code\|loc\|coverage\|dependencies\|crates\|modules]` | Project identity card and subpages (`--refresh` re-collects) |
| `ops run-before-commit [install]` | Pre-commit hook runner (`--changed-only` skips when nothing is staged) |
| `ops run-before-push [install]` | Pre-push hook runner (skips a delete-only or empty push) |
| `ops sec` | Security scans via Trivy — secrets always, vulnerability/misconfig auto-selected by file types (`--skip`/`--force` to override). Fails closed: non-zero on findings, on a scan timeout, and when `--skip` leaves no scan to run. Each scan is bounded by a 10-minute timeout, overridable with `OPS_SEC_TIMEOUT_SECS=<seconds>`. Build/dependency directories are skipped at any depth by default (see the [skip list](#ops-sec-default-skip-list) below); `--no-default-skips` opts out |
| `ops trailing-whitespace` (`tw`) | Strip trailing whitespace in place; non-zero when files changed (pre-commit contract) |
| `ops end-of-file-fixer` (`eof`) | Ensure files end with exactly one newline; non-zero when files changed |
| `ops check-json` / `check-yaml` | Verify every JSON/YAML file parses (`--tracked` limits to git files; `--allow-json5` for JSON5) |
| `ops backlog task create/edit/list/view` | Manage `.backlog/` markdown tasks — a compatible subset of [Backlog.md](https://github.com/MrLesk/Backlog.md); see [docs/backlog.md](docs/backlog.md) |
| `ops backlog search` | Keyword search over tasks, with `--modified-file` filtering |

Global flags: `--dry-run` (preview the resolved plan), `--verbose` (full stderr on
failure), `--tap <file>` (capture raw output), `--raw` (inherit child stdio, no ops output).

Hook escape hatches: set `SKIP_OPS_RUN_BEFORE_PUSH` (or `SKIP_OPS_RUN_BEFORE_COMMIT`)
to `1`, `true`, `yes` or `on` — case-insensitive; anything else means "do not skip" —
to let a push or commit through without running the configured hook commands.

### Stack-gated CLI

| Command | Available on |
|---------|--------------|
| `ops deps` | Rust |
| `ops plans` | Terraform (plan summary tables) |
| `ops about coverage` / `dependencies` | Rust |
| `ops about loc` | Rust |
| `ops about crates` / `modules` | Rust, Go |

### Stack command baseline

Every supported stack ships the same 7-command contract via `ops init --commands`.
A `✓` means the command is active by default; `*` means it's emitted commented-out
as a suggestion you can uncomment and adjust.

| Command | Rust | Vite | Node | Go | Python | TF | Ansible | Java-M | Java-G |
|---------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| `fmt`       | ✓ (cargo fmt) | * (bunx prettier) | * (prettier)   | ✓ (go fmt) | ✓ (ruff format, key `format`) | ✓ (tf fmt) | * (ansible-lint --fix) | * (spotless) | * (spotless) |
| `lint`      | ✓ (cargo clippy, key `clippy`) | ✓ (bunx eslint) | ✓ (npm run lint) | ✓ (go vet, key `vet`) | ✓ (ruff check) | * (tflint) | ✓ (ansible-lint) | * (spotless/checkstyle) | * (spotless/checkstyle) |
| `build`     | ✓ | ✓ (bunx vite build) | ✓ | ✓ | * (python -m build) | * (terraform plan) | * (galaxy build) | ✓ | ✓ |
| `test`      | ✓ | ✓ (bunx vitest run) | ✓ | ✓ | ✓ (pytest) | * (terraform test) | * (molecule test) | ✓ | ✓ |
| `clean`     | ✓ (cargo clean) | ✓ (rm node_modules dist) | ✓ (rm node_modules dist) | ✓ (go clean) | ✓ (rm caches) | ✓ (rm .terraform) | ✓ (sh -c rm .ansible *.retry) | ✓ | ✓ |
| `verify`    | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `qa`        | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

The Vite stack also ships a `typecheck` command (`bunx tsc -b --noEmit`) wired into its `verify`,
and is detected before Node via `vite.config.*` so Vite/TypeScript projects get type-aware defaults.

The Rust stack default goes beyond the contract: it also ships `next` / `next-ignored`
(cargo-nextest; nextest does not run doctests), `test-doc` for those doctests, and a
`qa-next` composite (alias `qax`) that runs the test legs through nextest. The Rust `qa`
runs `deps`, `test`, `test-ignored`, `test-doc`, and `sec` — `sec` requires the
[Trivy](https://trivy.dev) CLI on `PATH`.

#### `ops sec` default skip list

Every Trivy scan `ops sec` runs skips these directories at any depth, plus
`.git` — build output is generated artefact, not source: slow to walk, noisy
to scan, and it races the builds producing it (Trivy aborts when a file
vanishes mid-walk). `--no-default-skips` opts out; `--dry-run` previews the
exact patterns passed to Trivy.

| Stack | Skipped by default |
|-------|--------------------|
| Rust | `target` |
| Node / Vite | `node_modules` |
| Python | `.venv`, `venv`, `__pycache__` |
| Go | `vendor` |
| Terraform | `.terraform` |
| Java (Maven) | `target` |
| Java (Gradle) | `.gradle` |
| Ansible | — (collection/role caches default to `$HOME/.ansible`, not the repo) |

The generic names — `build` (Gradle, Python) and `dist` (Node, Vite,
Python) — are plausible checked-in source paths too, so they are never
skipped by name. A `build/` or `dist/` is skipped only where a manifest of
a stack that generates it sits in the parent directory (a `build/` beside
`build.gradle`, a `dist/` beside `package.json`), and Trivy receives those
as exact discovered paths rather than a blanket `**/build` — so a
checked-in `services/build` full of Dockerfiles is both detected and
scanned.

The list is shared with `ops sec`'s own detection walk, so detection and
scanning always agree on what counts as build output, and nested workspaces
are covered the same as the top level — a `fuzz/target` Cargo workspace skips
exactly like `target/`.

Commented suggestions show up verbatim when you run `ops init --commands`, so you can
opt in by uncommenting, or remap to the tool your project actually uses.

### Stack parity matrix

Rust is the reference implementation; the other stacks are data providers compiled
into the same binary via `ops-extension`. Parity gaps are feature scope, not
separate language rewrites.

Stack flavors currently shipped:

- **Rust** — cargo (workspaces supported)
- **Go** — go modules / `go.work`
- **Node** — package.json (pnpm/yarn/npm workspaces)
- **Java-Maven** — pom.xml with `<modules>` multi-module support
- **Java-Gradle** — Gradle with `settings.gradle(.kts)` subprojects
- **Python + uv** — `pyproject.toml` (PEP 621) with uv workspace members from `[tool.uv.workspace]` / `uv.lock`. A generic Python flavor (poetry, pip/setuptools, pdm, etc.) is **not yet implemented**.

| Area                                                | Rust | Go | Java-M | Java-G | Node | Python+uv |
|-----------------------------------------------------|:---:|:---:|:---:|:---:|:---:|:---:|
| CLI core (`init`, `theme`, `extension`, hooks)      | ✓   | ✓   | ✓   | ✓   | ✓   | ✓      |
| 7-command contract (fmt/lint/build/test/clean/verify/qa) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `project_identity` provider                         | ✓   | ✓   | ✓   | ✓   | ✓   | ✓      |
| Module *count* on identity card                     | ✓   | ✓   | ✓ (modules) | ✓ (subprojects) | ✓ | ✓ |
| `project_units` provider (`about modules` subpage)  | ✓   | ✓   | ✗   | ✗   | ✓   | ✓ (uv workspace only) |
| `about code` (tokei LOC, feature-gated)             | ✓   | ✓   | ✓   | ✓   | ✓   | ✓      |
| `about loc` (production/test/example split)         | ✓   | ✗   | ✗   | ✗   | ✗   | ✗      |
| `about coverage` (cargo llvm-cov)                   | ✓   | ✗   | ✗   | ✗   | ✗   | ✗      |
| `about dependencies` / `ops deps`                   | ✓   | ✗   | ✗   | ✗   | ✗   | ✗      |

Ranked by closeness to Rust parity: **Node** and **Python+uv** (identity + units + baseline CLI), **Go** (~90%, weaker units provider), **Java-Maven** / **Java-Gradle** (identity + module counts, but no `project_units` provider yet for the `about modules` subpage).

Rust-only extensions: `deps`, `cargo-toml`, `cargo-update`, `metadata`, `test-coverage`, `rust-loc`. `about code` is stack-agnostic (tokei scans any language) and only gated by the compile-time `tokei` feature on the `ops` binary; `about coverage` and `about dependencies` are Rust-only because their providers shell out to `cargo llvm-cov` / cargo metadata.

`about coverage` (and `about --refresh`, which re-collects coverage data) requires external tools that `ops` does not install for you:

```sh
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

When they are missing, the coverage warning/error includes these same install commands as a hint.

`about code` and `about loc` answer different questions and are not expected to agree: tokei reports every language but has no model for test versus production code, while `rust-loc` parses only `.rs` files and splits `#[cfg(test)]` blocks out of the file that contains them, counting doc comments separately from ordinary ones. On a non-Rust workspace `ops about loc` prints `No Rust LOC data available.`

#### Not yet implemented

- Generic Python stack (non-uv: poetry, pip/setuptools, pdm, hatch)
- Java `project_units` provider — module *counts* already surface on the main about card (Maven modules, Gradle subprojects), but `ops about modules` can't list them per-unit until a `project_units` provider ships

## Features

- **Zero config** — works out of the box with sensible defaults; `ops init` and friends scaffold the rest
- **Declarative commands** — define commands and command groups in TOML
- **Themed output** — step lines with timing; switch between themes easily
- **Extension architecture** — compile-time extensions; build your own ops
- **Parallel execution** — run command groups concurrently with `parallel = true`
- **Backlog tasks** — native `.backlog` markdown task management, output-compatible with the Backlog.md CLI (`--json` envelopes included), so existing tooling keeps working

## Contributing

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Only `feat` and `fix` commits trigger a release.

```bash
git commit -m "feat: add new feature"
git commit -m "fix: resolve bug"
```

See [docs/releasing.md](docs/releasing.md) for the full release workflow.

## Documentation

- [Releasing](docs/releasing.md) — automated releases, conventional commits, Homebrew tap
- [Visual Components](docs/components.md) — step icons, error boxes, theme comparison
- [Backlog tasks](docs/backlog.md) — `ops backlog` command reference, file format, and output contracts

## License

Apache-2.0
