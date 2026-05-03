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

- Rust edition is 2021. - Treat clippy warnings as errors. - After changing any `*.rs` file, run `ops verify` and `ops qa`,. - If those commands report errors or warnings, fix them and rerun the same gate.

## Common Commands

- Build: `cargo build --all-targets`
- Run: `cargo run -- <subcommand>` such as `cargo run -- verify`
- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Test: `cargo test`
- Full local gate: `ops verify qa install`

## Code Map

- `crates/core/src/config/`: TOML config parsing and theme config types.
- `crates/core/src/stack.rs`: stack detection and embedded default command templates.
- `crates/core/src/output.rs`: step line data types and display width behavior.
- `crates/theme/src/lib.rs`: `StepLineTheme` and configurable themes.
- `crates/runner/src/command/`: command execution engine and event stream.
- `crates/runner/src/display.rs`: progress rendering with `indicatif`.
- `crates/extension/src/lib.rs`: extension, command registry, data registry, context APIs.
- `crates/cli/src/theme_cmd.rs`: theme management CLI.
- `extensions/`: generic extensions.
- `extensions-<stack>/`: Each stack have its own code folder, e.g. extensions-java.

## Docs

- Releasing: `docs/releasing.md`
- Visual components and theme comparison: `docs/components.md`