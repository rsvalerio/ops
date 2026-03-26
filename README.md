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

# Run a command group
ops verify

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
commands = ["build", "clippy", "test"]
parallel = false
fail_fast = true

[commands.lint]
commands = ["fmt", "clippy", "check"]
parallel = true
```

Config is merged in order: built-in defaults → global config → local `.ops.toml` → env. When run inside a project with a detected stack (e.g. Rust), `ops init` pre-fills stack-specific commands.

## Commands

| Command | Description |
|---------|-------------|
| `ops <name>` | Run a configured command or command group |
| `ops init` | Create `.ops.toml` (use `--force` to overwrite) |
| `ops new-command` | Add a new command from a command line string |
| `ops theme list\|select` | List or select output themes |
| `ops extension list\|show` | List compiled-in extensions |
| `ops about` | Project identity card (Rust stacks) |
| `ops dashboard` | Project health dashboard (Rust stacks) |
| `ops tools list\|check\|install` | Manage dev tools (Rust stacks) |

## Features

- **Zero config** — works out of the box with sensible defaults; `ops init` and othere to scaffold the rest
- **Declarative commands** — define commands and command groups in TOML
- **Themed output** — step lines with timing; switch between themes easily
- **Extension architecture** — compile-time extensions; build your own ops
- **Parallel execution** — run command groups concurrently with `parallel = true`

### Backlog

- Review codebase looking for bad design, high cognitive load and lack of rust idioms and best practices
- Support conventional commit related commands: check git-cliff and cocogitto
- Support release related commands: check cargo-dist and go-releaser
- Make the about page "themed"
- Make the dashboard page "themed"
- Make the about page stack agnostic, with abstractions, each stack fill it up
- Make the dashboard page stack agnostic, with abstractions, each stack fill it up

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

## License

Apache-2.0
