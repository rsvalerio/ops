# Stack default command mappings

When no local `[commands]` override exists, `ops` merges **embedded stack defaults** from `crates/core/src/.default.<stack>.ops.toml` (wired in `crates/core/src/stack.rs`). Detection uses manifest files in the workspace (for example `Cargo.toml` for **rust**, `package.json` for **node**).

The **generic** stack has **no** embedded commands; define everything in `.ops.toml` or `.ops.d/*.toml`.

Below, **exec** lines are `program` plus `args` from config. **Composite** commands list child command names in order; see each stack’s `parallel` / `fail_fast` in the TOML for scheduling.

---

## rust (`Cargo.toml`)

| Command | Maps to |
| --- | --- |
| `fmt` | `cargo fmt --all` |
| `check` | `cargo check --all --all-features` |
| `clippy` | `cargo clippy --all --all-features -- -D warnings` |
| `lint` | alias → `clippy` |
| `build` | `cargo build --all --all-features` |
| `test` | `cargo test --all --all-features` |
| `test-ignored` | `cargo test --all --all-features -- --ignored` |
| `clean` | `cargo clean` |
| `verify` | composite: `fmt`, `check`, `clippy`, `build`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (parallel, fail-fast) |
| `qa` | composite: `deps`, `test`, `test-ignored` (parallel, fail-fast) |

**`deps`:** not defined in the embedded TOML; it is supplied by the **Rust `deps` extension** when built in. That command runs dependency health checks (notably `cargo upgrade --dry-run` and `cargo deny check`); see `extensions-rust/deps`.

---

## node (`package.json`)

| Command | Maps to |
| --- | --- |
| `install` | `npm install` |
| `build` | `npm run build` |
| `test` | `npm test` |
| `lint` | `npm run lint` |
| `verify` | composite: `install`, `lint`, `build`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast) |
| `qa` | composite: `test` (sequential, fail-fast) |

Suggested `fmt` and `clean` commands exist only as **commented** templates in the default TOML.

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
| `verify` | composite: `install`, `typecheck`, `lint`, `build`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast) |
| `qa` | composite: `test` (sequential, fail-fast) |

Suggested `fmt` (`bunx prettier --write .`), `preview` (`bunx vite preview`), and `clean` commands exist only as **commented** templates in the default TOML.

---

## go (`go.mod`)

| Command | Maps to |
| --- | --- |
| `fmt` | `go fmt ./...` |
| `vet` | `go vet ./...` |
| `lint` | alias → `vet` |
| `build` | `go build ./...` |
| `test` | `go test ./...` |
| `clean` | `go clean ./...` |
| `verify` | composite: `fmt`, `build`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast) |
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
| `verify` | composite: `fmt`, `lint`, `type`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast) |
| `qa` | composite: `test` (sequential, fail-fast) |

A suggested `build` (`uv build`) is commented in the default TOML.

---

## terraform (`main.tf` or `terraform.tf`)

| Command | Maps to |
| --- | --- |
| `init` | `terraform init` |
| `fmt` | `terraform fmt -recursive` |
| `validate` | `terraform validate` |
| `plan` | `terraform plan` |
| `verify` | composite: `fmt`, `validate`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast) |
| `qa` | composite: `plan` (sequential, fail-fast) |

Suggested `lint` (`tflint`), `build`, `test`, and `clean` are commented templates.

---

## ansible (`site.yml`, `playbook.yml`, or `ansible.cfg`)

| Command | Maps to |
| --- | --- |
| `lint` | `ansible-lint` |
| `check` | `ansible-playbook --check site.yml` |
| `verify` | composite: `lint`, `check`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast) |
| `qa` | composite: `check` (sequential, fail-fast) |

Suggested `fmt`, `build`, `test`, and `clean` are commented templates.

---

## java-maven (`pom.xml`)

Uses `./mvnw` (Maven wrapper).

| Command | Maps to |
| --- | --- |
| `compile` | `./mvnw compile` |
| `build` | `./mvnw package -DskipTests` |
| `test` | `./mvnw test` |
| `clean` | `./mvnw clean` |
| `verify` | composite: `compile`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast) |
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
| `verify` | composite: `compile`, `trailing-whitespace`, `end-of-file-fixer`, `check-json`, `check-yaml` (sequential, fail-fast) |
| `qa` | composite: `test` (sequential, fail-fast) |

Suggested `fmt` / `lint` (e.g. Spotless) are commented templates.

---

## Overrides and drift

Local `.ops.toml`, `~/.config/ops/config.toml`, and fragments under `.ops.d/` can **replace or extend** these names. If behavior differs from this page, the **effective config** in your repo is authoritative; this document describes **upstream defaults** only.
