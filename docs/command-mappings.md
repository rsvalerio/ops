# Stack default command mappings

When no local `[commands]` override exists, `ops` merges **embedded stack defaults** from `crates/core/src/.default.<stack>.ops.toml` (wired in `crates/core/src/stack/`). Detection uses manifest files in the workspace (for example `Cargo.toml` for **rust**, `package.json` for **node**).

To append steps to a default without shadowing the whole list, use an `[extend.<name>]` section in `.ops.toml` — `commands = ["coverage"]` for composites, `args = ["--locked"]` for exec commands (inserted before any `--` separator) — see [Extending existing commands](configuration.md#extending-existing-commands).

The **generic** stack has **no** embedded commands; define everything in `.ops.toml` or `.ops.d/*.toml`.

Below, **exec** lines are `program` plus `args` from config. **Composite** commands list child command names in order; see each stack’s `parallel` / `fail_fast` in the TOML for scheduling.

---

## rust (`Cargo.toml`)

| Command | Maps to |
| --- | --- |
| `fmt` | `cargo fmt --all` |
| `fmt-check` | `cargo fmt --all -- --check` |
| `check` | `cargo check --workspace --all-features --all-targets` |
| `clippy` | `cargo clippy --workspace --all-features --all-targets -- -D warnings` |
| `lint` | alias → `clippy` |
| `build` | `cargo build --workspace --all-features --all-targets` |
| `test` | `cargo test --workspace --all-features` |
| `test-doc` | `cargo test --workspace --all-features --doc` |
| `test-ignored` | `cargo test --workspace --all-features -- --ignored` |
| `next` | `cargo nextest run --workspace --all-features` |
| `next-ignored` | `cargo nextest run --workspace --all-features --run-ignored ignored-only` |
| `clean` | `cargo clean` |
| `verify` (`verify-check`) | composite: `fmt-check`, `trailing-whitespace-check`, `end-of-file-fixer-check`, `clippy`, `build`, `check-json`, `check-yaml`, `doc` (parallel, fail-fast); fixes nothing in place |
| `verify-fix` | composite: `fmt`, `trailing-whitespace`, `end-of-file-fixer`, `clippy`, `build`, `check-json`, `check-yaml`, `doc` (staged parallel, fail-fast); rewrites files in place |
| `qa` | composite: `deps`, `test`, `test-doc`, `sec` (sequential, fail-fast) |
| `qa-next` (`qax`) | composite: `deps`, `next`, `test-doc`, `sec` (sequential, fail-fast) |

**`verify` only checks, in every stack:** each rewriter in `verify-fix` is
swapped for its non-mutating twin — `fmt-check` in the stack TOML (Rust, Go,
Terraform), `trailing-whitespace-check` and `end-of-file-fixer-check` built in
(they run the fixer with `--check`). A dirty tree fails the gate instead of
being repaired, so the same command is safe on a dev machine and in CI. Run
`verify-fix` to format and fix files in place. `verify-check`, the name of the
Rust check form before it became the default, is kept as an alias.

**`--locked`:** none of the defaults pass it, because `--locked` refuses to
create a missing `Cargo.lock`. Set `[cargo] locked = true` (or
`OPS__CARGO__LOCKED=true` in CI) to add it to every lockfile-resolving cargo
command — see [Locked cargo commands](configuration.md#locked-cargo-commands).

**`--all-targets` on `test`:** deliberately absent. For `cargo test` the flag
*disables* doctests ("Test all targets (does not include doctests)"), so adding
it for symmetry would silently drop doctest coverage.

**`verify-fix` is staged:** `fmt`, `trailing-whitespace` and `end-of-file-fixer`
rewrite files the checks read, so each is exclusive and runs alone, in that
order (`fmt` is marked in the stack TOML, the fixers in their definitions).
`clippy`, `build`, `check-json`, `check-yaml` and `doc` then run concurrently.
See [Exclusive steps in a parallel group](configuration.md#exclusive-steps-in-a-parallel-group).

**`check` is not in `verify`:** `build --all-targets` subsumes it and `clippy`
type-checks independently, so including it compiled the workspace a third time
under a third fingerprint. It remains available standalone.

**`deps`:** not defined in the embedded TOML; it is supplied by the **Rust `deps` extension** when built in. That command runs seven dependency health checks: compatible and breaking upgrades (`cargo upgrade --dry-run`), advisories, licenses, duplicate crates and sources (`cargo deny check`), and unused dependencies (`cargo machete`). `cargo-edit` and `cargo-deny` are required. `ops deps --check` is the CI-safe gate: only `cargo deny check` and (when installed) `cargo machete`, collected fresh without the data cache, so `cargo-edit` is not needed; it fails on the same findings. `cargo-machete` is optional: without it the Unused Dependencies row shows as skipped with the install hint. Unused dependencies are a warning and never fail the gate, because cargo-machete is heuristic; suppress a false positive with `[package.metadata.cargo-machete] ignored = ["name"]`. See `extensions-rust/deps`.

**`sec`:** also not defined in the embedded TOML; it is the built-in `ops sec` subcommand (Trivy security scans — secrets always, vulnerability/misconfig auto-selected). Requires the `trivy` CLI on `PATH`.

**`next` / `next-ignored` vs `test` / `test-ignored`:** the `next*` commands run the same legs through cargo-nextest, which is faster but does not run doctests — hence `test-doc` in both `qa` and `qa-next`. Requires `cargo nextest` installed.

---

## node (`package.json`)

| Command | Maps to |
| --- | --- |
| `install` | `npm install` |
| `build` | `npm run build` |
| `test` | `npm test` |
| `lint` | `npm run lint` |
| `clean` | `rm -rf node_modules dist` |
| `verify` | composite: `install`, `lint`, `build`, `trailing-whitespace-check`, `end-of-file-fixer-check`, `check-json`, `check-yaml` (sequential, fail-fast); fixes nothing in place |
| `verify-fix` | composite: `install`, `lint`, `build`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast); rewrites files in place |
| `qa` | composite: `test` (sequential, fail-fast) |

A suggested `fmt` command exists only as a **commented** template in the default TOML.

---

## vite (`vite.config.{ts,js,mjs,mts,cjs,cts}`)

Detected **before** node, since every Vite project also ships a `package.json`. Commands invoke the toolchain directly through `bunx` so they work even when matching `package.json` scripts are absent; override `program` (e.g. `npx`, `pnpm dlx`) in `.ops.toml` to switch runner.

| Command | Maps to |
| --- | --- |
| `install` | `bun install` |
| `typecheck` | `bunx tsc -b --noEmit` |
| `build` | `bunx vite build` |
| `lint` | `bunx eslint .` |
| `test` | `bunx vitest run` |
| `clean` | `rm -rf node_modules dist` |
| `verify` | composite: `install`, `typecheck`, `lint`, `build`, `trailing-whitespace-check`, `end-of-file-fixer-check`, `check-json`, `check-yaml` (sequential, fail-fast); fixes nothing in place |
| `verify-fix` | composite: `install`, `typecheck`, `lint`, `build`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast); rewrites files in place |
| `qa` | composite: `test` (sequential, fail-fast) |

Suggested `fmt` (`bunx prettier --write .`) and `preview` (`bunx vite preview`) commands exist only as **commented** templates in the default TOML.

---

## go (`go.mod`)

| Command | Maps to |
| --- | --- |
| `fmt` | `go fmt ./...` |
| `fmt-check` | `gofmt -l` over the files `go list ./...` reports (what `go fmt ./...` formats), failing if it lists any file or errors |
| `vet` | `go vet ./...` |
| `lint` | alias → `vet` |
| `build` | `go build ./...` |
| `test` | `go test ./...` |
| `clean` | `go clean ./...` |
| `verify` | composite: `fmt-check`, `build`, `trailing-whitespace-check`, `end-of-file-fixer-check`, `check-json`, `check-yaml` (sequential, fail-fast); fixes nothing in place |
| `verify-fix` | composite: `fmt`, `build`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast); rewrites files in place |
| `qa` | composite: `test`, `vet` (sequential, fail-fast) |

---

## python (`pyproject.toml`, `setup.py`, or `requirements.txt`)

| Command | Maps to |
| --- | --- |
| `sync` | `uv sync --extra dev` |
| `ruff-fix` | `uv run ruff check --fix .` |
| `black-fmt` | `uv run black .` |
| `fmt` | composite: `ruff-fix`, then `black-fmt` (sequential, fail-fast) |
| `ruff` | `uv run ruff check .` |
| `black` | `uv run black --check .` |
| `lint` | composite: `ruff`, `black` (parallel, fail-fast) |
| `type` | `uv run pyright` |
| `test` | `uv run pytest -q` |
| `clean` | `rm -rf .pytest_cache .ruff_cache .pyright build dist` |
| `verify` | composite: `lint`, `type`, `trailing-whitespace-check`, `end-of-file-fixer-check`, `check-json`, `check-yaml` (sequential, fail-fast); fixes nothing in place |
| `verify-fix` | composite: `fmt`, `lint`, `type`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast); rewrites files in place |
| `qa` | composite: `test` (sequential, fail-fast) |

A suggested `build` (`uv build`) is commented in the default TOML.

---

## terraform (`main.tf` or `terraform.tf`)

| Command | Maps to |
| --- | --- |
| `tf-init` | `terraform init` (not `init`: `ops init` is the builtin that scaffolds `.ops.toml`) |
| `fmt` | `terraform fmt -recursive` |
| `fmt-check` | `terraform fmt -check -recursive` |
| `validate` | `terraform validate` |
| `plan` | `terraform plan` |
| `clean` | `rm -rf .terraform` (keeps `.terraform.lock.hcl`) |
| `verify` | composite: `fmt-check`, `validate`, `trailing-whitespace-check`, `end-of-file-fixer-check`, `check-json`, `check-yaml` (sequential, fail-fast); fixes nothing in place |
| `verify-fix` | composite: `fmt`, `validate`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast); rewrites files in place |
| `qa` | composite: `plan` (sequential, fail-fast) |

Suggested `lint` (`tflint`), `build`, and `test` are commented templates.

---

## ansible (`site.yml`, `playbook.yml`, or `ansible.cfg`)

| Command | Maps to |
| --- | --- |
| `lint` | `ansible-lint` |
| `check` | `ansible-playbook --check site.yml` |
| `clean` | `sh -c 'rm -rf .ansible *.retry'` (shell expands the glob; the runner execs without one) |
| `verify` | composite: `lint`, `check`, `trailing-whitespace-check`, `end-of-file-fixer-check`, `check-json`, `check-yaml` (sequential, fail-fast); fixes nothing in place |
| `verify-fix` | composite: `lint`, `check`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast); rewrites files in place |
| `qa` | composite: `check` (sequential, fail-fast) |

Suggested `fmt`, `build`, and `test` are commented templates.

---

## java-maven (`pom.xml`)

Uses `./mvnw` (Maven wrapper).

| Command | Maps to |
| --- | --- |
| `compile` | `./mvnw compile` |
| `build` | `./mvnw package -DskipTests` |
| `test` | `./mvnw test` |
| `clean` | `./mvnw clean` |
| `verify` | composite: `compile`, `trailing-whitespace-check`, `end-of-file-fixer-check`, `check-json`, `check-yaml` (sequential, fail-fast); fixes nothing in place |
| `verify-fix` | composite: `compile`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast); rewrites files in place |
| `qa` | composite: `test` (sequential, fail-fast) |

Suggested `fmt` / `lint` (e.g. Spotless) are commented templates.

---

## java-gradle (`build.gradle` or `build.gradle.kts`)

Uses `./gradlew` (Gradle wrapper).

| Command | Maps to |
| --- | --- |
| `compile` | `./gradlew compileJava` |
| `build` | `./gradlew build -x test` |
| `test` | `./gradlew test` |
| `clean` | `./gradlew clean` |
| `verify` | composite: `compile`, `trailing-whitespace-check`, `end-of-file-fixer-check`, `check-json`, `check-yaml` (sequential, fail-fast); fixes nothing in place |
| `verify-fix` | composite: `compile`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast); rewrites files in place |
| `qa` | composite: `test` (sequential, fail-fast) |

Suggested `fmt` / `lint` (e.g. Spotless) are commented templates.

---

## Overrides and drift

Local `.ops.toml`, `~/.config/ops/config.toml`, and fragments under `.ops.d/` can **replace or extend** these names. If behavior differs from this page, the **effective config** in your repo is authoritative; this document describes **upstream defaults** only.
