# cargo-ops

An opinionated, batteries-included Rust development CLI. Zero config, maximum quality.

## Installation

### Homebrew (macOS and Linux)

```bash
# 1. oneliner 
brew install rsvalerio/tap/ops

# 2. add tap then install 
brew tap rsvalerio/tap
brew install ops
```

### From source

> When running locally (no crates going to cates.io)

```bash
cargo install cargo-ops
```

## Configuration

Create a `.ops.toml` file in your project root (or run `cargo ops init`):

```toml
[output]
theme = "classic"   # "classic" (default) or "compact" or custom theme name
columns = 80        # line width for step lines
show_error_detail = true  # show error details below failed steps

[commands.build]
program = "cargo"
args = ["build", "--all-targets"]

[commands.clippy]
program = "cargo"
args = ["clippy", "--all-targets", "--", "-D", "warnings"]

[commands.test]
program = "cargo"
args = ["test"]

[commands.verify]
commands = ["build", "clippy", "test"]
parallel = false
fail_fast = true   # stop on first failure (default: true)

[commands.lint]
commands = ["fmt", "clippy", "check"]
parallel = true
```

Commands come from merged config: internal default (when no local file) → global config → local `.ops.toml` → env. Run `cargo ops init` to create a `.ops.toml`; when run inside a project with a detected stack (e.g. Rust with `Cargo.toml`), the file is pre-filled with that stack's default commands so you can run `cargo ops build`, `cargo ops verify`, etc. immediately. Use `cargo ops init --force` to overwrite an existing file.

## Documentation

- **[Releasing](docs/releasing.md)** - Automated releases, conventional commits, and Homebrew tap setup
- **[Visual Components](docs/components.md)** - Step icons, error boxes, theme comparison

## Contributing

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Only `feat` and `fix` commits trigger a release; all other types are included in the next changelog.

```bash
git commit -m "feat: add new feature"
git commit -m "fix: resolve bug"

# Or use cocogitto for guided semantic commits
cog commit feat "add new feature"
cog commit fix "resolve bug"
```

See [docs/releasing.md](docs/releasing.md) for the full commit type reference, breaking changes, and release workflow.

## Features

- **Theme-Based Output** - Plain-text step lines (classic: full command + dots + time; compact: step id + time); customize with `cargo ops theme list/select`
- **Declarative Commands** - Define commands in TOML config
- **Configurable Columns** - Set line width via `output.columns` (no runtime change)
- **Extension Architecture** - Extensible via compile-time extensions (commands and data providers)
- **Metadata Collection** - Optional data-provider extensions (feature-gated) with DuckDB storage
- **Zero Config** - Works out of the box with sensible defaults

## License

Apache-2.0
