# AGENTS.md

Instructions for AI coding agents working on this project.

**Every time a rust file (`*.rs`) is changed, make sure to run `ops verify; ops install`. If error or warning are reported, fix it and run verify and install again.**

## Project overview

`ops` is an opinionated, batteries-included development CLI.

**Key concepts:**
- Commands are defined in `.ops.toml` (or from internal default when no file). Run `ops init` to create `.ops.toml`; when a stack is detected (e.g. Rust via `Cargo.toml`), the written file is merged with that stack's default commands (from embedded `.default.<stack>.ops.toml`).
- Commands can be **exec** (run a program) or **composite** (run multiple commands)
- Extension trait for registering commands and data providers
- CLI output: theme-based plain text to stdout/stderr (themed step lines, streaming output, summary); themes: classic (default), compact; configurable columns

## Setup commands

- Build: `cargo build`
- Run: `cargo run -- <subcommand>` (e.g. `cargo run -- build`, `cargo run -- verify`)
- Install it locally: `cargo install --path crates/cli` then use `ops <command>`
- Initialize config: `ops init` creates `.ops.toml` in the current directory, merging in stack default commands when a stack is detected; `ops init --force` overwrites existing

## Build and test

- **Build:** `cargo build` or `cargo build --all-targets`
- **Tests:** `cargo test`
- **Lint:** `cargo clippy --all-targets -- -D warnings`
- **Format:** `cargo fmt` (check only: `cargo fmt -- --check`)
- **Full verify:** `ops verify` runs build → clippy → test in sequence

**Every time a rust file (`*.rs`) is changed, make sure to run `cargo install --path crates/cli --force` after passing `cargo clippy --all-targets -- -D warnings`.**

## Code style

- Rust edition 2021
- Clippy with `-D warnings` (treat warnings as errors)
- Fix all clippy and format issues before considering a change done
- Follow existing workspace structure:
  - `crates/core/src/config/mod.rs` - TOML config parsing (output.theme, output.columns, show_error_detail)
  - `crates/core/src/output.rs` - Step line data types (StepLine, StepStatus, ErrorDetail) and display width
  - `crates/core/src/style.rs` - Visual style constants
  - `crates/theme/src/lib.rs` - StepLineTheme trait, ConfigurableTheme
  - `crates/core/src/config/theme_types.rs` - ThemeConfig struct and classic/compact factory methods
  - `crates/runner/src/command/mod.rs` - CommandRunner execution engine, StepResult, RunnerEvent stream
  - `crates/runner/src/display.rs` - ProgressDisplay for step rendering with indicatif
  - `crates/extension/src/lib.rs` - Extension trait, CommandRegistry, DataRegistry, Context
  - `crates/cli/src/theme_cmd.rs` - Theme management CLI (list, select)
  - `extensions/` - Optional extensions (duckdb, tokei)
  - `extensions-rust/` - Rust-specific extensions (about, cargo-toml, metadata, tools, etc.)

## Testing instructions

- Run the full suite: `cargo test`
- Tests live next to the code they cover (`#[cfg(test)] mod tests` in the same file)
- Some tests use `#[tokio::test]` for async command execution
- After changes, run `cargo test` and fix any failures
- Add or update tests for new behavior

## Before committing and after changes to *.rs files

Run these commands and fix any issues:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

Then install it locally: `cargo install --path crates/cli --force`

## Documentation

- **Releasing:** [docs/releasing.md](docs/releasing.md) - automated releases, conventional commits, and Homebrew tap setup
- **Visual components:** [docs/components.md](docs/components.md) - step icons, error boxes, theme comparison

## Configuration

Configuration is merged (later overrides earlier):
1. Internal default (base config; when a stack is detected at runtime or at `init` time, stack default commands from embedded `.default.<stack>.ops.toml` are also loaded)
2. Global config in `~/.config/ops/config.toml` (optional)
3. Local `.ops.toml` in current directory (optional; overrides internal default when present)
4. `.ops.d/*.toml` files (sorted alphabetically; good for separating themes, commands)
5. Environment variables `CARGO_OPS_*`

`ops init` writes a merged template: base config plus the detected stack's default commands (so e.g. in a Rust project the generated `.ops.toml` already contains `[commands.build]`, `[commands.clippy]`, `[commands.verify]`, etc.).

### Split config with `.ops.d/`

For better organization, place additional config files in `.ops.d/`:

```
.ops.toml           # main config
.ops.d/themes.toml  # custom themes
.ops.d/commands.toml # project-specific commands
```

Files are merged in alphabetical order after `.ops.toml`. Each file uses the same format.

Example `.ops.toml`:

```toml
[output]
theme = "classic"   # "classic" (default) or "compact" or custom theme name
columns = 80        # line width for step lines
show_error_detail = true  # show error details below failed steps

[commands.build]
program = "cargo"
args = ["build", "--all-targets"]

[commands.verify]
commands = ["build", "clippy", "test"]
parallel = false
fail_fast = true   # stop on first failure (default: true)
```
