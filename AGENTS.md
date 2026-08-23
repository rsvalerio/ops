# AGENTS.md

Instructions for AI coding agents working on this project.

`ops` is an opinionated, batteries-included development CLI. Commands are defined in
`.ops.toml` or internal stack defaults and can be exec commands or composite commands.

## Core Workflow

- Don’t assume. Don’t hide confusion. Surface tradeoffs.
- Minimum code that solves the problem. Nothing speculative.
- Touch only what you must. Clean up only your own mess.
- Define success criteria. Loop until verified.
- Prefer existing project patterns over new abstractions.
- Keep root guidance short; add scoped `AGENTS.md` files near code that needs local rules.
- Put tests next to the code they cover with `#[cfg(test)] mod tests` when practical.
- Add or update tests for new behavior.

- Rust edition is 2021.
- Treat clippy warnings as errors.
- After changing any `*.rs` file, run `ops verify` and `ops qa`. If those commands report errors or warnings, fix them and rerun the same gate. (`qa` ends with `sec`, which needs the Trivy CLI on `PATH`.)

## Rust implementation guardrails

For any non-trivial Rust change, read the `code-review-rust` skill *before*
editing and follow its rules as acceptance criteria. Do not file backlog tasks
during implementation — that mode is for formal reviews only.

Run `cargo fmt`, `cargo clippy --all-targets --workspace -- -D warnings`, and
`cargo nextest run --workspace --all-features` (plus `cargo test --workspace
--doc` — nextest does not run doctests) before declaring the change done.

Lint levels are centralized in `[workspace.lints]`; no crate sets its own. To
silence a lint, grant the exception at the narrowest scope that works and write
the reason next to it — see `docs/clippy.md`. Never run `cargo clippy --fix`
without reading the diff: it has silently deleted load-bearing code here.

## Common Commands

- Build: `cargo build --all-targets`
- Run: `cargo run -- <subcommand>` such as `cargo run -- verify`
- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Test: `ops next` (nextest; doctests via `ops test-doc`, or `ops qa-next` for the full nextest gate)
- Full local gate: `ops verify qa install`

## DuckDB prebuilt library

The workspace does not compile the DuckDB amalgamation (`bundled` is off).
Linking builds instead use a prebuilt library, fetched and checksum-verified by
`scripts/fetch-duckdb.sh` (pinned in `scripts/duckdb-pins.txt`; cached under
`target/duckdb-prebuilt/`). Before `cargo build`, `cargo test`, `cargo
nextest run`, or `ops verify qa`:

    eval "$(scripts/fetch-duckdb.sh)"

`cargo check`, `cargo clippy`, and `cargo fmt` work without it. If you skip it,
ops-duckdb's build script warns, and linking fails with
`library not found: duckdb`. The first fetch needs network; it is cached per
DuckDB version. Details: `docs/duckdb-prebuilt-lib.md`.

## Code Map

- `crates/core/src/config/`: TOML config parsing and theme config types.
- `crates/core/src/stack/`: stack detection (`detect.rs`) and the embedded `.default.<stack>.ops.toml` command templates.
- `crates/core/src/output.rs`: step line data types and display width behavior.
- `crates/theme/src/lib.rs`: `StepLineTheme` and configurable themes.
- `crates/runner/src/command/`: command execution engine and event stream.
- `crates/runner/src/display.rs`: progress rendering with `indicatif`.
- `crates/extension/src/lib.rs`: extension, command registry, data registry, context APIs.
- `crates/cli/src/theme_cmd.rs`: theme management CLI.
- `crates/cli/src/sec_cmd.rs`: Trivy-based security scans (`ops sec`).
- `extensions/`: generic extensions.
- `extensions-<stack>/`: Each stack have its own code folder, e.g. extensions-java.

## Docs

- Releasing: `docs/releasing.md`
- Stack default command mappings: `docs/command-mappings.md`
- Visual components and theme comparison: `docs/components.md`
- Lint policy, exceptions and how to add one: `docs/clippy.md`
- DuckDB build-time reduction, option A (try first): `docs/duckdb-prebuilt-lib.md`
- DuckDB build-time reduction, option B: `docs/duckdb-cli-backend.md`
- DuckDB alternatives (SQLite, plain Rust): `docs/duckdb-alternatives.md`
