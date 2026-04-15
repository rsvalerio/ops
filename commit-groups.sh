#!/usr/bin/env bash
set -e

echo "Group 1 — refactor(extensions-rust/about): decompose lib.rs into focused modules"
git add \
  extensions-rust/about/src/lib.rs \
  extensions-rust/about/src/cards.rs \
  extensions-rust/about/src/dashboard.rs \
  extensions-rust/about/src/format.rs \
  extensions-rust/about/src/identity.rs \
  extensions-rust/about/src/query.rs \
  extensions-rust/about/src/text_util.rs
git commit -m "refactor(extensions-rust/about): decompose lib.rs into focused modules"

echo "Group 2 — refactor(extensions-rust/deps): reduce lib.rs and extract metadata"
git add \
  extensions-rust/deps/src/lib.rs \
  extensions-rust/metadata/src/ingestor.rs
git commit -m "refactor(extensions-rust/deps): extract metadata ingestor module"

echo "Group 3 — refactor(extensions-rust/test-coverage): extract ingestor module"
git add extensions-rust/test-coverage/src/ingestor.rs
git commit -m "refactor(extensions-rust/test-coverage): extract ingestor module"

echo "Group 4 — refactor(extensions-java/about): simplify about extension"
git add extensions-java/about/src/lib.rs
git commit -m "refactor(extensions-java/about): simplify about extension"

echo "Group 5 — refactor(extensions/about and extensions-go/about): update extensions"
git add \
  extensions-go/about/src/lib.rs \
  extensions/about/src/lib.rs
git commit -m "refactor(extensions): update about extensions"

echo "Group 6 — refactor(extensions/duckdb): remove sql.rs and simplify"
git add \
  extensions/duckdb/src/sql.rs \
  extensions/duckdb/src/lib.rs \
  extensions/duckdb/src/ingestor.rs
git commit -m "refactor(extensions/duckdb): remove sql.rs and simplify lib.rs"

echo "Group 7 — refactor(theme): simplify lib.rs"
git add crates/theme/src/lib.rs
git commit -m "refactor(theme): simplify lib.rs"

echo "Group 8 — refactor(core): add config merge and update identity/stack"
git add \
  crates/core/src/config/merge.rs \
  crates/core/src/lib.rs \
  crates/core/src/project_identity.rs \
  crates/core/src/stack.rs
git commit -m "refactor(core): add config merge and update identity/stack"

echo "Group 9 — refactor(cli): simplify main.rs and extension commands"
git add \
  crates/cli/src/main.rs \
  crates/cli/src/extension_cmd.rs
git commit -m "refactor(cli): simplify main.rs and extension commands"

echo "Group 10 — refactor(runner): update command execution and display"
git add \
  crates/runner/src/command/mod.rs \
  crates/runner/src/command/exec.rs \
  crates/runner/src/command/tests.rs \
  crates/runner/src/display.rs
git commit -m "refactor(runner): update command execution and display"

echo "Group 11 — test(extension): update test suite"
git add crates/extension/src/tests.rs
git commit -m "test(extension): update test suite"

echo "Group 12 — refactor(extensions/hooks): update hook implementations"
git add \
  extensions/run-before-commit/Cargo.toml \
  extensions/run-before-commit/src/lib.rs \
  extensions/run-before-push/Cargo.toml \
  extensions/run-before-push/src/lib.rs
git commit -m "refactor(extensions/hooks): update run-before-commit and run-before-push"

echo "Group 13 — chore(cargo-toml): fix extension cargo parsing"
git add extensions-rust/cargo-toml/src/lib.rs
git commit -m "chore(cargo-toml): fix extension cargo parsing"

echo "Group 14 — chore(deps): update dependencies and security config"
git add \
  Cargo.lock \
  Cargo.toml \
  deny.toml
git commit -m "chore(deps): update dependencies and security config"

echo "Group 15 — chore(backlog): update task descriptions"
git add \
  ".backlog/tasks/task-0023 - Hook-extension-crates-are-near-identical-copies.md" \
  ".backlog/tasks/task-0024 - dir_name-utility-duplicated-across-3-about-extensions.md"
git commit -m "chore(backlog): update task descriptions"

echo "Group 16 — chore(backlog): add new quality and security analysis tasks"
git add ".backlog/tasks/task-002"[5-9]*.md
git commit -m "chore(backlog): add quality and security analysis tasks"

echo "Group 17 — chore(backlog): add architecture and duplication analysis tasks"
git add ".backlog/tasks/task-003"*.md
git commit -m "chore(backlog): add architecture and duplication analysis tasks"

echo "Group 18 — chore(backlog): add remaining analysis and refactor tasks"
git add ".backlog/tasks/task-004"*.md ".backlog/tasks/task-005"*.md
git commit -m "chore(backlog): add remaining analysis and refactor tasks"

echo "Group 19 — chore(backlog): archive completed tasks"
git add ".backlog/archive/"
git commit -m "chore(backlog): archive completed tasks"
