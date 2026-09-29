# Contributing to ops

Thanks for taking the time to contribute. Bug reports, ideas, docs fixes and code
are all welcome. This guide covers how to ask, how to report, and how to get a
change merged.

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md). By taking
part you agree to uphold it.

## Table of Contents

- [I have a question](#i-have-a-question)
- [Reporting bugs](#reporting-bugs)
- [Suggesting enhancements](#suggesting-enhancements)
- [Your first code contribution](#your-first-code-contribution)
  - [Development setup](#development-setup)
  - [Making a change](#making-a-change)
  - [Quality gates](#quality-gates)
- [Commit messages](#commit-messages)
- [Pull requests](#pull-requests)
- [Releases](#releases)
- [AI coding agents](#ai-coding-agents)

## I have a question

Read the [README](README.md) and the [docs](docs/) first. If they do not answer
it, search the [issues](https://github.com/rsvalerio/ops/issues), then open a
new one with as much context as you can: what you ran, what you expected, and
your `ops --version`, OS and stack.

## Reporting bugs

Please do not report security vulnerabilities in public issues. Follow
[SECURITY.md](SECURITY.md) instead.

For any other bug, open an [issue](https://github.com/rsvalerio/ops/issues/new) with:

- `ops --version`, your OS and architecture, and how you installed `ops`
- the detected stack, and the relevant `.ops.toml` snippet if you have one
- the exact command, what happened, and what you expected
- output from a re-run with `-v` / `--verbose` if a step failed

A minimal reproduction, such as a small repository or a config and one command,
makes the fix much faster.

## Suggesting enhancements

Open an [issue](https://github.com/rsvalerio/ops/issues/new) that describes the
problem before the solution: what you are trying to do, how you do it today, and
why that falls short. `ops` is deliberately opinionated, so explain how the
change fits a stack's defaults, or why it belongs in `.ops.toml` instead.

## Your first code contribution

### Development setup

You need a Rust toolchain (rustup recommended). The repository gates itself
with `ops`, so install it from your checkout first:

```bash
git clone https://github.com/rsvalerio/ops.git
cd ops
cargo install --path crates/cli --all-features
```

The full gate also needs [cargo-nextest](https://nexte.st), and
[Trivy](https://trivy.dev) on `PATH` for `ops sec`.

Optionally, install the git hooks so the commit and push gates run
automatically:

```bash
ops run-before-commit install
ops run-before-push install
```

### Making a change

- Branch from `main` with a conventional prefix, such as `feat/…`, `fix/…` or `docs/…`.
- Keep the change focused: touch only what the change needs, and prefer
  the existing patterns in the code over new abstractions.
- Put tests next to the code they cover (`#[cfg(test)] mod tests`), and add or
  update tests for new behavior.
- Update the docs when behavior changes. The README stays short;
  [docs/configuration.md](docs/configuration.md) and
  [docs/commands.md](docs/commands.md) hold the reference.
- Lint levels live in `[workspace.lints]` in the root `Cargo.toml`. To silence
  a lint, follow [docs/clippy.md](docs/clippy.md).

### Quality gates

Run both gates before you push; CI runs the same checks:

```bash
ops verify   # check-only: fmt, whitespace, clippy, build, doc (ops verify-fix repairs)
ops qa       # deps, test, test-doc, sec
```

Without `ops`, these cargo commands cover the format, lint and test legs only:

```bash
cargo fmt
cargo clippy --all-targets --workspace -- -D warnings
cargo nextest run --workspace --all-features
cargo test --workspace --doc
```

The dependency and security checks have no single cargo equivalent, so run
`ops deps` and `ops sec` for those.

## Commit messages

Commits follow [Conventional Commits](https://www.conventionalcommits.org/),
because the release version and the changelog are generated from them:

```text
<type>(<scope>): <description>
```

| Type | Use it for | Release |
|------|-----------|---------|
| `feat` | a new feature | minor |
| `fix` | a bug fix | patch |
| `docs`, `test`, `refactor`, `perf`, `build`, `ci`, `chore`, `style` | everything else | none |

The scope is the affected area, such as `cli`, `runner`, `about` or `backlog`.
Mark a breaking change with `!` (`feat(config)!: …`) and a `BREAKING CHANGE:`
footer. Put the `!` in the PR title too; see
[docs/releasing.md](docs/releasing.md) for why.

## Pull requests

- Open the PR against `main` and describe what changed and why.
- `main` is protected: the CI checks must pass, review threads must be
  resolved, and commits must be signed.
- An automated review (CodeRabbit) comments on every PR. Address its findings,
  or reply with the reason you are not taking one.

## Releases

You do not cut releases by hand. When a `feat` or `fix` lands on `main`, the
Bump workflow updates the version and `CHANGELOG.md`, tags the release, and
triggers the release build. That build publishes binaries, the shell installer,
the Homebrew formula and the apt packages. See
[docs/releasing.md](docs/releasing.md).

## AI coding agents

Agents follow [AGENTS.md](AGENTS.md), which extends this guide with the
repository's working rules and code map.
