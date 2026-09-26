# ops

An opinionated, batteries-included development CLI operator: define commands and
command groups once in TOML and run them with themed, parallel, fail-fast output.

## Features

- **Zero config** — works out of the box with sensible defaults; `ops init` and friends scaffold the rest
- **Declarative commands** — define commands and command groups in TOML
- **Themed output** — step lines with timing; switch between themes easily
- **Extension architecture** — compile-time extensions; build your own ops
- **Parallel execution** — run command groups concurrently with `parallel = true`
- **Backlog tasks** — native `.backlog` markdown task management, output-compatible with the Backlog.md CLI (`--json` envelopes included), so existing tooling keeps working

## Getting Started

These instructions give you a copy of the project up and running on your local
machine for development and testing purposes.

### Prerequisites

- A Rust toolchain (rustup recommended) to build from source
- Optional: the [Trivy](https://trivy.dev) CLI on `PATH`, used by `ops sec`
- Optional: [cargo-nextest](https://nexte.st) and `cargo-llvm-cov` for the Rust test/coverage commands

### Installing

Homebrew (macOS and Linux):

```bash
brew install rsvalerio/tap/ops
```

From a checkout of this repository:

```bash
cargo install --path crates/cli
```

## Usage

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

### Configuration

Create a `.ops.toml` file in your project root (or run `ops init`):

```toml
[output]
theme = "classic"        # "classic" (default) or "compact"
columns = 80             # line width for step lines (omit to auto-size: 90% of terminal width, 80 without one)
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

#### Extending existing commands

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

- Composites (`commands = [...]`) extend with `commands`; exec commands extend with `args`, and matrix commands also with `matrix.<key> = [...]` (see [Running one command over a matrix](#running-one-command-over-a-matrix)). Using the wrong key for the target's kind, extending an undefined name, or an entry that sets none of `commands`, `args`, `help`, `category`, `matrix` is a load error naming the target.
- `help = "..."` replaces the target's help text, and `category = "..."` replaces its category (either kind of target). Given across config layers, the last layer that sets one wins.
- Without a `help` override, appending `commands` to a composite that has help extends the help to name the appended commands (`"...; then extra-a, extra-b"`), so `ops --help` can never quietly understate what `ops <cmd> --dry-run` runs. The same guard applies across layers: a layer that appends commands without setting `help` gets its commands named in whatever help is in effect, so a later command-only layer cannot hide behind an earlier override. A composite without help needs nothing — its help fallback already renders the materialized command list.
- Appended `args` land **before the target's first `--` separator** when one is present, otherwise at the end of the args. Cargo commands like the Rust `clippy ... -- -D warnings` pass everything after `--` to the wrapped tool, so inserting before it keeps `--locked` a cargo flag instead of silently turning it into a lint flag.
- A locally redefined command wins: `[extend.verify]` appends to *your* `[commands.verify]` if you defined one, otherwise to the stack default.
- Extends concatenate across config layers, so `.ops.d/*.toml` fragments stack on top of `.ops.toml` appends.
- Extending controls list order only, not execution order. Each appended command keeps the `exclusive` flag of its own definition. In a sequential group it runs after the earlier steps. In a parallel group (see below), an appended non-exclusive command joins the final stage and may run concurrently with the earlier non-exclusive steps. Mark it `exclusive = true` if it must not overlap them.

#### Cloning existing commands

To define a command as a variant of an existing one — typically a stack default — without copying (and going stale on) its whole spec, use `clone`:

```toml
[commands.fuzz-clippy]
clone = "clippy"

[extend.fuzz-clippy]
args = ["--manifest-path", "fuzz/Cargo.toml"]
```

`fuzz-clippy` is a copy of the resolved Rust `clippy` default (program and args included), and `[extend.fuzz-clippy]` adds the fuzz-specific flag — so the variant tracks the default's flags as they evolve. Composites clone the same way (`clone = "verify"` copies the `commands` list). The extras are materialized at load time, so `ops --dry-run fuzz-clippy` shows the resolved program and args. Rules:

- The source resolves like an `[extend]` target: your `[commands]` entry if you defined one, otherwise the detected stack's default. Extension-registered commands cannot be cloned — they register after config load — and naming one is an unknown-source load error.
- Scalar fields beside `clone` (`help`, `category`, `aliases`, and for exec sources `env`, `cwd`, `timeout_secs`, `exclusive`, `strategy`) override the copy; fields left unset keep the source's value. Given maps and lists replace the copy (`env` replaces, it does not merge). `aliases` are the exception: they are never inherited — a clone with no `aliases` has none, because inheriting the source's would either collide at load or silently redirect the source's alias to the clone. `program`, `args` and `commands` beside `clone` are load errors — extra args go through `[extend.<name>]`.
- A clone copies the source **before** the source's own `[extend.<source>]` applies: extends stay per-name, so the clone never inherits them. `[extend.<clone>]` applies to the materialized copy.
- Unknown sources, clone cycles (including self-clones; non-cyclic clone-of-clone chains do resolve), cloning into an existing stack-default name, and exec-only fields beside a composite source are load errors naming the command and the source.

#### Command groups and scheduling

A command with a `commands = [...]` list is a *group* (composite). Groups may
reference other groups, and each group runs under its own `parallel` flag:

- **Sequential group (`parallel = false`, the default):** each entry runs as its
  own plan, one after another, under that entry's own schedule — exactly what
  typing the entries on the command line does. A parallel child group runs its
  steps concurrently (split into stages at `exclusive` steps, below); a
  sequential child runs its own entries one at a time. So
  `pre-release = ["verify", "deps", ...]` means `ops verify deps ...`: `verify`
  keeps the staged parallel schedule it gets when invoked directly, and a hook
  group such as `run-before-commit = ["verify", ...]` runs `verify` in parallel
  rather than one step at a time.
- **Parallel group (`parallel = true`):** the whole tree under it is one flat
  plan. Every group inside it must also declare `parallel = true` — a nested
  `parallel = false` group's steps would run concurrently despite the flag, so
  the config is rejected with an error naming both groups:

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

To fix, set `verify.parallel = false` so each entry keeps its own schedule (the
nested `fixers` group then runs in whatever order it declares itself), or keep
it parallel and mark the steps that must not overlap `exclusive = true` (below).
Inside one parallel plan every group must also declare the same `fail_fast`.

`fail_fast` follows the same boundary rule:

- Within one parallel plan, every group must agree on `fail_fast` (the plan is
  scheduled as one unit).
- Across a sequential group's entries, values may differ: each entry's own
  `fail_fast` governs its steps, and the sequential group's own `fail_fast`
  governs the sequence — when false, every entry runs regardless of failures;
  when true (the default), a failing entry stops the entries after it, unless
  that entry itself declares `fail_fast = false` (it asked to run through its
  own failures, so the sequence continues past it). This mirrors what naming
  several commands on one invocation (`ops run verify qa`) does, where each
  name keeps its own flags.

#### Exclusive steps in a parallel group

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

#### Running one command over a matrix

To run one exec command once per value — say `cargo doc` per published crate,
each invocation separate so cargo resolves every crate's default features on
its own — give it a `strategy`, modelled on GitHub Actions:

```toml
[commands.doc-default]
program = "cargo"
args = ["doc", "--no-deps", "-p", "${matrix.crate}"]
env = { CARGO_TARGET_DIR = "${CARGO_TARGET_DIR:-target}/doc-default" }

[commands.doc-default.strategy]
matrix = { crate = ["dbsec-core", "dbsec", "dbsec-pgwire", "dbsec-vault", "dbsec-derive"] }
max_parallel = 1     # the cells share a target dir; running them together only queues on its lock
fail_fast = false    # run every crate and report every failure

[extend.verify]
commands = ["doc-default"]
```

`ops doc-default` runs `cargo doc --no-deps -p dbsec-core`, then `-p dbsec`, and
so on, and shows one row per cell, labelled by its values
(`doc-default [crate=dbsec-core]`). When cells fail, each failed row shows its
error and the step fails naming every failed cell. `ops --dry-run doc-default`
lists every cell with its expanded program and args.

A matrix command is **one step** to the plan around it:

- `exclusive` covers the whole matrix: no sibling step overlaps any cell.
- It succeeds only when every cell succeeds.
- `strategy.max_parallel` (default: every cell at once; cells always count
  against the same `OPS_MAX_PARALLEL` process cap as the plan's other steps)
  and `strategy.fail_fast` (default `true`: the first
  failing cell cancels the rest) govern only the cells. They never conflict
  with the enclosing group's flags. The `fail_fast` agreement rule applies to
  groups, not to a matrix, so the `fail_fast = false` matrix above can sit
  inside the parallel, fail-fast `verify`. It runs all five crates, and
  `verify` treats the matrix's overall failure like any failing step.
- `--raw` runs the cells one after another, like everything else.

Cells follow GitHub Actions semantics:

- Several keys form the Cartesian product. Cells are ordered by key name, then
  by value order.
- `exclude = [{ os = "mac", crate = "dbsec" }]` drops every cell that matches
  all of an entry's pairs.
- Each `include` entry is merged into every cell it does not contradict on a
  matrix key. When it fits none, it becomes a cell of its own. `include` and
  `exclude` live inside `matrix`, as in GitHub Actions, so neither name can be
  a key.

`${matrix.<key>}` is substituted in `args`, `env` values and `cwd`, before
`${VAR}` expansion. It never falls back to the environment. These are load
errors naming the command: a key that some cell does not define
(`${matrix.crat}`), a `${matrix.*}` reference on a command with no `strategy`,
and a reference in `program`. A matrix is capped at 256 cells.

A clone copies the source's `strategy`, and a `strategy` beside `clone`
replaces the copied one wholesale. `[extend.<name>] matrix.<key> = [...]`
appends values to an existing key (a key the matrix does not have is a load
error). `matrix.include` / `matrix.exclude` there append entries. A matrix
over a composite is not supported.

### Commands

#### Stack-agnostic CLI (same on every stack)

| Command | Description |
|---------|-------------|
| `ops <name>` | Run a configured command or command group |
| `ops init` | Create `.ops.toml` (minimal by default; `--force` to overwrite; `--output`/`--themes`/`--commands` add those sections, with stack-detected commands under `--commands`) |
| `ops new-command` | Add a new command from a command line string |
| `ops import-makefile` | Import Makefile targets as `.ops.toml` commands (interactive picker) |
| `ops theme list\|select` | List or select output themes |
| `ops extension list\|show` | List compiled-in extensions |
| `ops about [setup\|code\|loc\|coverage\|dependencies\|crates\|modules\|backlog]` | Project identity card and subpages (`--refresh` re-collects) |
| `ops run-before-commit [install]` | Pre-commit hook runner (`--changed-only` skips when nothing is staged) |
| `ops run-before-push [install]` | Pre-push hook runner (skips a delete-only or empty push) |
| `ops sec` | Security scans via Trivy — secrets always, vulnerability/misconfig auto-selected by file types (`--skip`/`--force` to override). Fails closed: non-zero on findings, on a scan timeout, and when `--skip` leaves no scan to run. Each scan is bounded by a 10-minute timeout, overridable with `OPS_SEC_TIMEOUT_SECS=<seconds>`. Build/dependency directories are skipped at any depth by default (see the [skip list](#ops-sec-default-skip-list) below); `--no-default-skips` opts out. See [scan root, ignore file, dev deps](#ops-sec-scan-root-ignore-file-and-dev-dependencies) for `--repo`, `.trivyignore.yaml` and `--no-dev-deps` |
| `ops trailing-whitespace` (`tw`) | Strip trailing whitespace in place; non-zero when files changed (pre-commit contract) |
| `ops end-of-file-fixer` (`eof`) | Ensure files end with exactly one newline; non-zero when files changed |
| `ops check-json` / `check-yaml` | Verify every JSON/YAML file parses (`--tracked` limits to git files; `--allow-json5` for JSON5) |
| `ops backlog init` | Bootstrap the backlog: a `[backlog]` section in `.ops.toml` (or `backlog.config.yml` with `--backlog.md`) plus the tasks tree; also run by `ops init` |
| `ops backlog task create/edit/list/view` | Manage `.backlog/` markdown tasks — a compatible subset of [Backlog.md](https://github.com/MrLesk/Backlog.md); see [docs/backlog.md](docs/backlog.md) |
| `ops backlog search` | Keyword search over tasks, with `--modified-file` filtering |
| `ops backlog wave list\|members\|migrate` | Inspect code-review waves and their member tasks |
| `ops backlog cleanup` | Move terminal-status tasks older than a cutoff to `completed/` (`--older-than <days>`, `--dry-run` to preview) |
| `ops backlog create-review-tasks` | Create `review-request-<date>-<n>` backlog tasks with one review subtask per workspace target (`--dry-run` to preview) |

Global flags: `--dry-run` (preview the resolved plan), `--verbose` (full stderr on
failure), `--tap <file>` (capture raw output), `--raw` (inherit child stdio, no ops output).

Hook escape hatches: set `SKIP_OPS_RUN_BEFORE_PUSH` (or `SKIP_OPS_RUN_BEFORE_COMMIT`)
to `1`, `true`, `yes` or `on` — case-insensitive; anything else means "do not skip" —
to let a push or commit through without running the configured hook commands.

#### Stack-gated CLI

| Command | Available on |
|---------|--------------|
| `ops deps` | Rust |
| `ops plans` | Terraform (plan summary tables) |
| `ops about coverage` / `dependencies` | Rust |
| `ops about loc` | Rust |
| `ops about crates` / `modules` | Rust, Go, Node, Python (uv), Java-M, Java-G |

#### Stack command baseline

Every supported stack ships the same 7-command contract via `ops init --commands`.
A `✓` means the command is active by default; `*` means it's emitted commented-out
as a suggestion you can uncomment and adjust.

| Command | Rust | Vite | Node | Go | Python | TF | Ansible | Java-M | Java-G |
|---------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| `fmt`       | ✓ (cargo fmt) | * (bunx prettier) | * (prettier)   | ✓ (go fmt) | ✓ (ruff --fix then black) | ✓ (tf fmt) | * (ansible-lint --fix) | * (spotless) | * (spotless) |
| `lint`      | ✓ (cargo clippy, key `clippy`) | ✓ (bunx eslint) | ✓ (npm run lint) | ✓ (go vet, key `vet`) | ✓ (ruff check) | * (tflint) | ✓ (ansible-lint) | * (spotless/checkstyle) | * (spotless/checkstyle) |
| `build`     | ✓ | ✓ (bunx vite build) | ✓ | ✓ | * (uv build) | * (terraform plan) | * (galaxy build) | ✓ | ✓ |
| `test`      | ✓ | ✓ (bunx vitest run) | ✓ | ✓ | ✓ (pytest) | * (terraform test) | * (molecule test) | ✓ | ✓ |
| `clean`     | ✓ (cargo clean) | ✓ (rm node_modules dist) | ✓ (rm node_modules dist) | ✓ (go clean) | ✓ (rm caches) | ✓ (rm .terraform) | ✓ (sh -c rm .ansible *.retry) | ✓ | ✓ |
| `verify`    | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `qa`        | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

The Vite stack also ships a `typecheck` command (`bunx tsc -b --noEmit`) wired into its `verify`,
and is detected before Node via `vite.config.*` so Vite/TypeScript projects get type-aware defaults.

The Rust stack default goes beyond the contract: it also ships `next` / `next-ignored`
(cargo-nextest; nextest does not run doctests), `test-doc` for those doctests, and a
`qa-next` composite (alias `qax`) that runs the test legs through nextest. The Rust `qa`
runs `deps`, `test`, `test-doc`, and `sec` — `sec` requires the
[Trivy](https://trivy.dev) CLI on `PATH`.

##### `ops sec` default skip list

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

##### `ops sec` scan root, ignore file and dev dependencies

- **Scan root.** `ops sec` scans the directory it runs in. Inside a
  subproject of a git repo (a monorepo whose `qa` runs `sec` in `backend/`
  and `frontend/`), files elsewhere in the repo are left out, so no file is
  scanned twice. The plan instead names every Dockerfile / Kubernetes /
  IaC file it leaves out — anywhere in the git toplevel outside the scan
  root and outside sibling directories with their own `.ops.toml` — and
  says how to include them: `ops sec --repo` scans the git toplevel (or run
  `ops sec` from the repo root, e.g. as a separate CI step).
- **Ignore file.** The first of `.trivyignore.yaml` and `.trivyignore`
  found at the scan root, then at the git toplevel, is passed to every scan
  via `--ignorefile`. The YAML format wins because it is the one that
  supports path-scoped rules. `--dry-run` names the file used, or says none
  was found.
- **Dev dependencies.** The vulnerability scan includes dev dependencies
  (`--include-dev-deps`) by default: build tooling and test runners run on
  developer and CI machines, which is what a security gate is for. Trivy
  supports this for npm, yarn and gradle only, so other ecosystems are
  unaffected. `--no-dev-deps` opts out.

Commented suggestions show up verbatim when you run `ops init --commands`, so you can
opt in by uncommenting, or remap to the tool your project actually uses.

#### Stack parity matrix

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
| `project_units` provider (`about modules` subpage)  | ✓   | ✓   | ✓ (modules) | ✓ (subprojects) | ✓   | ✓ (uv workspace only) |
| `about code` (tokei LOC, feature-gated)             | ✓   | ✓   | ✓   | ✓   | ✓   | ✓      |
| `about loc` (production/test/example split)         | ✓   | ✗   | ✗   | ✗   | ✗   | ✗      |
| `about coverage` (cargo llvm-cov)                   | ✓   | ✗   | ✗   | ✗   | ✗   | ✗      |
| `about dependencies` / `ops deps`                   | ✓   | ✗   | ✗   | ✗   | ✗   | ✗      |

Ranked by closeness to Rust parity: **Node**, **Python+uv**, **Java-Maven** and **Java-Gradle** (identity + units + baseline CLI), **Go** (~90%, weaker units provider).

Rust-only extensions: `deps`, `cargo-toml`, `cargo-update`, `metadata`, `coverage` (crate `test-coverage`, behind the `coverage` compile feature), `rust-loc`, `about-rust`, `create-review-tasks-rust`. `about code` is stack-agnostic (tokei scans any language) and gated by the compile-time `sqlite` feature on the `ops` binary (both `tokei` and `stack-rust` enable it; the tokei collector itself only compiles under `tokei`); `about coverage` and `about dependencies` are Rust-only because their providers shell out to `cargo llvm-cov` / cargo metadata.

`about coverage` (and `about --refresh`, which re-collects coverage data) requires external tools that `ops` does not install for you:

```sh
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

When they are missing, the coverage warning/error includes these same install commands as a hint.

`about code` and `about loc` answer different questions and are not expected to agree: tokei reports every language but has no model for test versus production code, while `rust-loc` parses only `.rs` files and splits `#[cfg(test)]` blocks out of the file that contains them, counting doc comments separately from ordinary ones. On a non-Rust workspace `ops about loc` prints `No Rust LOC data available.`

## Running the tests

The project gates itself with its own commands:

```bash
ops verify   # fmt, check, clippy, build
ops qa       # deps, test, test-doc, sec (qa needs the Trivy CLI on PATH)
```

The raw cargo invocations for the format, lint, and test legs (the `check`,
`build`, `deps`, and `sec` gates have no direct cargo equivalent here):

```bash
cargo fmt
cargo clippy --all-targets --workspace -- -D warnings
cargo nextest run --workspace --all-features   # nextest does not run doctests
cargo test --workspace --doc                   # doctests
```

## Roadmap

- Generic Python stack (non-uv: poetry, pip/setuptools, pdm, hatch)
- Per-group scheduling boundaries — "run these groups in order, but let the
  steps inside one group run together" (see the note under
  [Command groups and scheduling](#command-groups-and-scheduling))

## Built With

- [clap](https://docs.rs/clap) — CLI parsing
- [indicatif](https://docs.rs/indicatif) — progress rendering
- [rusqlite](https://docs.rs/rusqlite) — embedded analytics store (bundled SQLite)
- [tokei](https://docs.rs/tokei) — LOC counting
- [Trivy](https://trivy.dev) — security scans (external CLI)
- [cargo-nextest](https://nexte.st) — test runner (external CLI)

## Contributing

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Only `feat` and `fix` commits trigger a release.

```bash
git commit -m "feat: add new feature"
git commit -m "fix: resolve bug"
```

See [AGENTS.md](AGENTS.md) for the working rules this repo follows (gates, lint
policy, code map) and [docs/releasing.md](docs/releasing.md) for the full
release workflow.

## Versioning

We use [Semantic Versioning](http://semver.org/). For the versions available,
see the [tags on this repository](https://github.com/rsvalerio/ops/tags).
Releases are automated from conventional commits — see
[docs/releasing.md](docs/releasing.md).

## Authors

- **Rodrigo Valeri** — [rsvalerio](https://github.com/rsvalerio)

See also the list of
[contributors](https://github.com/rsvalerio/ops/contributors)
who participated in this project.

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE)
file for details.

## Acknowledgments

- [Backlog.md](https://github.com/MrLesk/Backlog.md) — the backlog command and
  file format `ops backlog` stays compatible with
- [pre-commit](https://pre-commit.com) — the trailing-whitespace and
  end-of-file-fixer hooks follow its exit-code contract
- [Conventional Commits](https://www.conventionalcommits.org/) — release automation

## Documentation

- [Releasing](docs/releasing.md) — automated releases, conventional commits, Homebrew tap
- [Visual Components](docs/components.md) — step icons, error boxes, theme comparison
- [Backlog tasks](docs/backlog.md) — `ops backlog` command reference, file format, and output contracts
