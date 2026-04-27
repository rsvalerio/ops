# AGENTS.md

Local guidance for `crates/core/src`.

## Configuration

Configuration is merged in this order, with later sources overriding earlier ones:

1. Internal base config plus detected stack defaults.
2. Global config in `~/.config/ops/config.toml`.
3. Local `.ops.toml` in the current directory.
4. `.ops.d/*.toml` files, sorted alphabetically.
5. Environment variables named `CARGO_OPS_*`.

`ops init` writes a merged template: base config plus the detected stack's default
commands. For a Rust project, the generated template should already include commands
such as `build`, `clippy`, `verify`, and `qa`.

## Stack Defaults

Per-stack defaults live in `.default.<stack>.ops.toml` and are embedded by
`stack.rs` with `include_str!`. Supported stacks are `rust`, `node`, `go`, `python`,
`terraform`, `ansible`, `java-maven`, and `java-gradle`.

Every stack ships the same 7-command baseline:

- `fmt`
- `lint`
- `build`
- `test`
- `clean`
- `verify`
- `qa`

If a stack has no obvious default for one of these commands, leave a commented-out
baseline suggestion in the template instead of inventing a weak default.

<important if="path=crates/core/src/.default.*.ops.toml">
- Keep every stack default TOML parseable.
- Preserve `verify` and `qa`; `stack.rs` tests enforce them for every stack.
- Prefer idiomatic command names such as Rust `clippy`, Go `vet`, and Python `format`.
- Aliases may satisfy baseline intent without renaming well-known tool commands.
</important>

<important if="path=crates/core/src/stack.rs">
- Keep stack metadata in one place when adding or changing stack detection.
- Add or update tests for stack parsing, detection, and default command invariants.
- New non-generic stacks need manifest files and an embedded `.default.<stack>.ops.toml`.
</important>
