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

Config is merged in order: built-in defaults → global config → local `.ops.toml` → env. When run inside a project with a detected stack (e.g. Rust), `ops init` pre-fills stack-specific commands.

## Commands

### Stack-agnostic CLI (same on every stack)

| Command | Description |
|---------|-------------|
| `ops <name>` | Run a configured command or command group |
| `ops init` | Create `.ops.toml` (use `--force` to overwrite; `--commands` emits stack defaults) |
| `ops new-command` | Add a new command from a command line string |
| `ops theme list\|select` | List or select output themes |
| `ops extension list\|show` | List compiled-in extensions |
| `ops about [setup\|code\|coverage\|dependencies\|crates\|modules]` | Project identity card and subpages |
| `ops run-before-commit [install]` | Pre-commit hook runner |
| `ops run-before-push [install]` | Pre-push hook runner |

### Stack-gated CLI

| Command | Available on |
|---------|--------------|
| `ops deps` | Rust |
| `ops tools list\|check\|install` | Rust |
| `ops about coverage` / `dependencies` | Rust |
| `ops about crates` / `modules` | Rust, Go |

### Stack command baseline

Every supported stack ships the same 7-command contract via `ops init --commands`.
A `✓` means the command is active by default; `*` means it's emitted commented-out
as a suggestion you can uncomment and adjust.

| Command | Rust | Node | Go | Python | TF | Ansible | Java-M | Java-G |
|---------|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| `fmt`       | ✓ (cargo fmt) | * (prettier)   | ✓ (go fmt) | ✓ (ruff format, key `format`) | ✓ (tf fmt) | * (ansible-lint --fix) | * (spotless) | * (spotless) |
| `lint`      | ✓ (cargo clippy, key `clippy`) | ✓ (npm run lint) | ✓ (go vet, key `vet`) | ✓ (ruff check) | * (tflint) | ✓ (ansible-lint) | * (spotless/checkstyle) | * (spotless/checkstyle) |
| `build`     | ✓ | ✓ | ✓ | * (python -m build) | * (terraform plan) | * (galaxy build) | ✓ | ✓ |
| `test`      | ✓ | ✓ | ✓ | ✓ (pytest) | * (terraform test) | * (molecule test) | ✓ | ✓ |
| `clean`     | ✓ (cargo clean) | * (rm node_modules dist) | ✓ (go clean) | * (rm caches) | * (rm .terraform) | * (rm .ansible) | ✓ | ✓ |
| `verify`    | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `qa`        | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

Commented suggestions show up verbatim when you run `ops init --commands`, so you can
opt in by uncommenting, or remap to the tool your project actually uses.

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
