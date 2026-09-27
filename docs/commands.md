# Commands

Every `ops` subcommand, which stacks it applies to, and the default commands each
stack ships. How to define your own is in [Configuration](configuration.md).

## Stack-agnostic CLI (same on every stack)

| Command | Description |
|---------|-------------|
| `ops <name>` | Run a configured command or command group |
| `ops init` | Create `.ops.toml` (minimal by default; `--force` to overwrite; `--output`/`--themes`/`--commands` add those sections, with stack-detected commands under `--commands`) |
| `ops explain <cmd>... [--json]` | Show the resolved plan without running anything: each composite's `parallel`/`fail_fast`, the stages a parallel group splits into at its `exclusive` steps, and each step's program, args, env, cwd and origin (stack default, config, `clone`, `[extend]`, extension, builtin). `--json` emits a versioned (`schemaVersion`) document. Named `explain` because the terraform stack ships a `plan` command |
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
| `ops backlog task create/edit/list/view` | Manage `.backlog/` markdown tasks — a compatible subset of [Backlog.md](https://github.com/MrLesk/Backlog.md); see [backlog.md](backlog.md) |
| `ops backlog search` | Keyword search over tasks, with `--modified-file` filtering |
| `ops backlog wave list\|members\|migrate` | Inspect code-review waves and their member tasks |
| `ops backlog cleanup` | Move terminal-status tasks older than a cutoff to `completed/` (`--older-than <days>`, `--dry-run` to preview) |
| `ops backlog create-review-tasks` | Create `review-request-<date>-<n>` backlog tasks with one review subtask per workspace target (`--dry-run` to preview) |

Global flags: `--dry-run` (preview the resolved plan — never executes: `run-before-commit` /
`run-before-push` print their plan, and backlog actions that cannot preview — `backlog init`,
`task create`/`edit`, `commit` and `wave create`/`claim`/`park` — refuse it with an error, as do the
builtins that write or run tools without a preview mode: `init`, `new-command`,
`import-makefile`, `trailing-whitespace`, `end-of-file-fixer`, `theme select`, `lock` /
`lock break`, `about` (except `about backlog`), `deps` and `plans`), `--verbose` (full stderr on
failure), `--tap <file>` (capture raw output), `--raw` (inherit child stdio, no ops output).

Hook escape hatches: set `SKIP_OPS_RUN_BEFORE_PUSH` (or `SKIP_OPS_RUN_BEFORE_COMMIT`)
to `1`, `true`, `yes` or `on` — case-insensitive; anything else means "do not skip" —
to let a push or commit through without running the configured hook commands.

## Stack-gated CLI

| Command | Available on |
|---------|--------------|
| `ops deps` | Rust |
| `ops plans` | Terraform (plan summary tables) |
| `ops about coverage` / `dependencies` | Rust |
| `ops about loc` | Rust |
| `ops about crates` / `modules` | Rust, Go, Node, Python (uv), Java-M, Java-G |
| `ops about machine` | any (build-relevant machine state; Linux and macOS) |

`ops about crates`, `loc` and `dependencies` take `--json` for a versioned
(`schemaVersion`) machine-readable document; `ops about machine --json`
likewise. `ops about dependencies --duplicates [--json]` lists crates locked
at two or more distinct versions (dev-only ones excluded unless
`--include-dev` is given), the direct
dependency pulling each older version, and whether a semver-compatible
update of it removes the duplicate (`cargo update --dry-run`; `Cargo.lock`
is never written). Like `cargo tree`, it counts only dependency edges
active on the host target; `--target <triple>` (repeatable) selects other
targets and `--target all` counts every edge. `ops deps` (cargo-deny `bans`)
flags multiple versions too, but not what pulls them in or whether an update
fixes them. `ops about machine` reports the effective `jobs`, rustc wrapper,
target dir, linker, rustflags and incremental setting, each with its source.

## Stack command baseline

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

### `ops sec` default skip list

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

### `ops sec` scan root, ignore file and dev dependencies

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

## Stack parity matrix

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
