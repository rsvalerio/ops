# AGENTS.md

Instructions for AI coding agents working on this project, in the
[agents.md](https://agents.md) format. Human contributors: start with
[CONTRIBUTING.md](CONTRIBUTING.md), which these rules extend.

## Project overview

`ops` is an opinionated, batteries-included development CLI. Commands are defined in
`.ops.toml` or internal stack defaults and can be exec commands or composite commands.
It is a Rust workspace (edition 2021): `crates/` holds the core, runner, theme,
extension API, backlog and CLI crates; `extensions/` and `extensions-<stack>/`
hold the compile-time extensions. See [Code map](#code-map).

## Core workflow

- Don’t assume. Don’t hide confusion. Surface tradeoffs.
- Minimum code that solves the problem. Nothing speculative.
- Touch only what you must. Clean up only your own mess.
- Define success criteria. Loop until verified.
- Prefer existing project patterns over new abstractions.
- Keep root guidance short; add scoped `AGENTS.md` files near code that needs local rules.

## Setup commands

- Build: `cargo build --all-targets`
- Run: `cargo run -- <subcommand>` such as `cargo run -- verify`
- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Test: `ops next` (nextest; doctests via `ops test-doc`, or `ops qa-next` for the full nextest gate)
- Full local gate: `ops verify qa install`

The embedded analytics engine is rusqlite with bundled SQLite
(`extensions/sqlite`, crate `ops-sqlite`). Builds need no link env — the
former prebuilt-libduckdb eval step is gone. The database file is the
disposable cache `target/ops/data.db` under the workspace root; ad-hoc
queries work with the stock `sqlite3` CLI. Migration notes:
`docs/duckdb-to-sqlite.md`.

## Code style

- Rust edition is 2021.
- Treat clippy warnings as errors.
- For any non-trivial Rust change, read the `code-review-rust` skill *before*
  editing and follow its rules as acceptance criteria. Do not file backlog tasks
  during implementation — that mode is for formal reviews only.
- Lint levels are centralized in `[workspace.lints]`; no crate sets its own. To
  silence a lint, grant the exception at the narrowest scope that works and write
  the reason next to it — see `docs/clippy.md`. Never run `cargo clippy --fix`
  without reading the diff: it has silently deleted load-bearing code here.

## Testing instructions

- Put tests next to the code they cover with `#[cfg(test)] mod tests` when practical.
- Add or update tests for new behavior.
- After changing any `*.rs` file, run `ops verify` and `ops qa`. If those commands
  report errors or warnings, fix them and rerun the same gate. (`qa` ends with
  `sec`, which needs the Trivy CLI on `PATH`.)
- Run `cargo fmt`, `cargo clippy --all-targets --workspace -- -D warnings`, and
  `cargo nextest run --workspace --all-features` (plus `cargo test --workspace
  --doc` — nextest does not run doctests) before declaring the change done.

## PR instructions

- Commits follow [Conventional Commits](https://www.conventionalcommits.org/);
  only `feat` and `fix` cut a release. See [CONTRIBUTING.md](CONTRIBUTING.md#commit-messages).
- Backlog tasks are managed with `ops backlog task ...` / `ops backlog search`
  (native, Backlog.md-compatible — see `docs/backlog.md`); prefer it over the
  external `backlog` CLI. Do not hand-edit task files: field types and marker
  layout are load-bearing for the triage and wave skills.

## Security considerations

Commands in `.ops.toml` run unsanitized by design, the same trust model as
`make` (see the README's [Security](README.md#security) section). Keep that
boundary explicit: `ops` itself must not follow symlinks out of the workspace,
leak secrets into output, or resolve programs from an unexpected `PATH`. Report
vulnerabilities as described in [SECURITY.md](SECURITY.md).

## Code map

- `crates/core/src/config/`: TOML config parsing and theme config types.
- `crates/core/src/stack/`: stack detection (`detect.rs`) and the embedded `.default.<stack>.ops.toml` command templates.
- `crates/core/src/output.rs`: step line data types and display width behavior.
- `crates/theme/src/`: `ConfigurableTheme` (`configurable.rs`) and the step-line theme types (`step_line_theme.rs`).
- `crates/runner/src/command/`: command execution engine and event stream.
- `crates/runner/src/display.rs`: progress rendering with `indicatif`.
- `crates/extension/src/lib.rs`: extension, command registry, data registry, context APIs.
- `crates/backlog/`: `.backlog` markdown task management (`ops backlog init`, `ops backlog task create/edit/list/view`, `ops backlog search`, `ops backlog wave list/members/overlap/claim/park/migrate`, `ops backlog commit`) — a Backlog.md-compatible subset; `model.rs` parses/writes the task files, `store.rs` scans and allocates ids, `render.rs` owns the output contracts. Backlog config resolves from `.ops.toml`'s `[backlog]` section (wins) or `backlog.config.yml` — see `docs/backlog.md`.
- `crates/cli/src/theme_cmd.rs`: theme management CLI.
- `crates/cli/src/sec_cmd.rs`: Trivy-based security scans (`ops sec`).
- `crates/cli/src/lock_cmd.rs`: `ops lock` — named `flock` locks under the common git dir, shared by all worktrees.
- `extensions-rust/foundation/`: the Rust foundation templates and the `ops init --rust` scaffold and drift check — see `docs/foundation.md`.
- `extensions/`: generic extensions.
- `extensions-<stack>/`: Each stack have its own code folder, e.g. extensions-java.

## Docs

- Configuration (`.ops.toml`, extend, clone, scheduling, matrix): `docs/configuration.md`
- Every subcommand, stack defaults and the parity matrix: `docs/commands.md`
- Backlog task management (`ops backlog`): `docs/backlog.md`
- Releasing: `docs/releasing.md`
- Stack default command mappings: `docs/command-mappings.md`
- Visual components and theme comparison: `docs/components.md`
- Lint policy, exceptions and how to add one: `docs/clippy.md`
- Rust foundation templates, scaffold and drift check (`ops init --rust`): `docs/foundation.md`
- DuckDB → SQLite migration notes: `docs/duckdb-to-sqlite.md`
