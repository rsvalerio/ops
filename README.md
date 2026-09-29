# ops

[![CI](https://github.com/rsvalerio/ops/actions/workflows/ci.yml/badge.svg)](https://github.com/rsvalerio/ops/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/rsvalerio/ops)](https://github.com/rsvalerio/ops/releases/latest)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg)](https://github.com/RichardLitt/standard-readme)

Batteries-included task runner for any stack

`ops` detects your project's stack and gives it a working set of commands, such as
`build`, `verify` and `qa`, with no configuration. It supports Rust, Vite, Node,
Go, Python (uv), Terraform, Ansible, Java (Maven and Gradle) and a generic
fallback. When the defaults are not enough, you define commands and command
groups once in `.ops.toml` and run them with themed, parallel, fail-fast output.
The same file drives your local gates, your git hooks and CI.

## Table of Contents

- [Security](#security)
- [Background](#background)
- [Install](#install)
  - [Dependencies](#dependencies)
  - [Updating](#updating)
- [Usage](#usage)
  - [CLI](#cli)
- [Configuration](#configuration)
- [Documentation](#documentation)
- [Development](#development)
- [Roadmap](#roadmap)
- [Maintainers](#maintainers)
- [Thanks](#thanks)
- [Contributing](#contributing)
- [License](#license)

## Security

`ops` runs the commands in `.ops.toml` as written, without sanitizing them. It
uses the same trust model as `make` or `npm run`: a project's config is trusted
code, so only run `ops` in directories you trust. To report a vulnerability, see
[SECURITY.md](SECURITY.md).

## Background

Every project needs the same few gates, like format, lint, build, test and
audit, but each stack spells them differently. Each repository also ends up
re-deriving them in a Makefile, in hook scripts and in CI. `ops` keeps one
opinionated definition per stack and lets a project extend it instead of
rewriting it.

- **Zero config**: stack detection and built-in defaults work out of the box, and `ops init` scaffolds the rest.
- **Declarative commands**: exec commands, composite groups, clones, `[extend]` layers and matrix commands, all in TOML.
- **Scheduling**: parallel groups, fail-fast, exclusive steps and nested plans, with one themed step line per command.
- **Git hooks**: `ops run-before-commit` and `ops run-before-push` run a configured gate from the installed hook.
- **Project insight**: `ops about` reports code statistics, crates or modules, dependencies, coverage, backlog health and build-machine state.
- **Backlog tasks**: native `.backlog` markdown task management, output-compatible with the [Backlog.md](https://github.com/MrLesk/Backlog.md) CLI.
- **Extension architecture**: compile-time extensions, so you can build your own `ops`.

`ops` is written in Rust and builds on [clap](https://docs.rs/clap) for CLI
parsing, [indicatif](https://docs.rs/indicatif) for progress rendering,
[rusqlite](https://docs.rs/rusqlite) with bundled SQLite for its analytics store,
and [tokei](https://docs.rs/tokei) for LOC counting.

## Install

Homebrew (macOS and Linux):

```bash
brew install rsvalerio/tap/ops
```

apt (Debian and Ubuntu, amd64 and arm64):

```bash
sudo apt update && sudo apt install -y curl gpg
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://rsvalerio.github.io/apt/public.key \
  | sudo gpg --dearmor -o /etc/apt/keyrings/rsvalerio.gpg
echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/rsvalerio.gpg] https://rsvalerio.github.io/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/rsvalerio.list
sudo apt update && sudo apt install ops
```

From a checkout of this repository:

```bash
cargo install --path crates/cli
```

### Dependencies

Building from source needs a Rust toolchain (rustup recommended). Some commands
call external tools that you install separately:

- [Trivy](https://trivy.dev) on `PATH` for `ops sec`, and so for any gate that includes it, such as this repository's `ops qa`
- [cargo-nextest](https://nexte.st) and `cargo-llvm-cov` for the Rust test and coverage commands
- Each stack's own toolchain (`cargo`, `go`, `bun`, `uv`, `terraform`, …) for that stack's default commands

### Updating

Use the tool you installed with: `brew upgrade ops`, `sudo apt update && sudo apt
upgrade`, or re-run `cargo install --path crates/cli` from an updated checkout.
Release notes are in [CHANGELOG.md](CHANGELOG.md).

## Usage

### CLI

```bash
# Initialize config for your project (auto-detects stack)
ops init

# Run a command
ops build

# Run static checks, fixing nothing (ops verify-fix formats and fixes in place)
ops verify

# Run tests and quality checks
ops qa

# Add a new command interactively
ops new-command "cargo fmt --check"
```

Run `ops --help` for the commands available in the current project, grouped by
category, including the ones defined in `.ops.toml`.

## Configuration

Create `.ops.toml` in your project root, or run `ops init`:

```toml
[commands.test]
program = "cargo"
args = ["test"]

[commands.verify]
commands = ["fmt", "clippy", "build"]
parallel = true
fail_fast = true
```

Config merges from built-in defaults, the global `~/.config/ops/config.toml`,
the local `.ops.toml`, `.ops.d/*.toml` fragments and `OPS__*` environment
variables, in that order. [docs/configuration.md](docs/configuration.md) covers
extending, cloning, scheduling, exclusive steps and matrix commands.

## Documentation

- [Configuration](docs/configuration.md): `.ops.toml`, extend and clone, command groups and scheduling, matrix commands
- [Commands](docs/commands.md): every subcommand, stack-gated commands, stack defaults and the parity matrix
- [Stack command mappings](docs/command-mappings.md): what each default command runs on each stack
- [Visual components](docs/components.md): step icons, error boxes, theme comparison
- [Backlog tasks](docs/backlog.md): the `ops backlog` command reference, file format and output contracts
- [Releasing](docs/releasing.md): automated releases from conventional commits, Homebrew and apt publishing
- [Lint policy](docs/clippy.md): the workspace Clippy policy and how to add an exception

## Development

The project gates itself with its own commands:

```bash
ops verify   # check-only: fmt, whitespace, clippy, build, doc (ops verify-fix repairs)
ops qa       # deps, test, test-doc, sec (qa needs the Trivy CLI on PATH)
```

`ops deps` checks upgrades, advisories, licenses, duplicate crates, sources and
unused dependencies. It needs `cargo-edit` and `cargo-deny`; the unused-dependencies
check also uses `cargo-machete` when it is installed, and is skipped otherwise.

The raw cargo invocations for the format, lint, and test legs (the `check`,
`build`, `deps`, and `sec` gates have no direct cargo equivalent here):

```bash
cargo fmt
cargo clippy --all-targets --workspace -- -D warnings
cargo nextest run --workspace --all-features   # nextest does not run doctests
cargo test --workspace --doc                   # doctests
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full workflow.

## Roadmap

- Generic Python stack (non-uv: poetry, pip/setuptools, pdm, hatch)
- Per-group scheduling boundaries: "run these groups in order, but let the
  steps inside one group run together" (see
  [Command groups and scheduling](docs/configuration.md#command-groups-and-scheduling))

## Maintainers

- [@rsvalerio](https://github.com/rsvalerio) (Rodrigo Valeri)

## Thanks

- [Backlog.md](https://github.com/MrLesk/Backlog.md): the backlog command and
  file format that `ops backlog` stays compatible with
- [pre-commit](https://pre-commit.com): the trailing-whitespace and
  end-of-file-fixer hooks follow its exit-code contract
- [Conventional Commits](https://www.conventionalcommits.org/) and
  [Cocogitto](https://docs.cocogitto.io): release automation
- [cargo-dist](https://github.com/axodotdev/cargo-dist): release builds and installers

## Contributing

Questions, bug reports and ideas are welcome in
[GitHub issues](https://github.com/rsvalerio/ops/issues), and PRs are accepted.
Commits follow [Conventional Commits](https://www.conventionalcommits.org/),
because releases are cut from them. Read [CONTRIBUTING.md](CONTRIBUTING.md)
before opening a PR. This project follows the
[Contributor Covenant](CODE_OF_CONDUCT.md).

## License

[Apache-2.0](LICENSE) © Rodrigo Valeri
