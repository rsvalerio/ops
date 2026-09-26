//! TASK-2277: `[commands.<name>.strategy]` end to end through
//! `load_config_at` — parsing, load-time validation, clone and extend.

use crate::config::{load_config_at, CommandSpec, Config, ExecCommandSpec};

/// The dbsec worked case from the task (trimmed to three crates).
const DOC_DEFAULT: &str = r#"
[commands.doc-default]
program = "cargo"
args = ["doc", "--no-deps", "-p", "${matrix.crate}"]
env = { CARGO_TARGET_DIR = "${CARGO_TARGET_DIR:-target}/doc-default" }

[commands.doc-default.strategy]
matrix = { crate = ["dbsec-core", "dbsec", "dbsec-vault"] }
max_parallel = 1
fail_fast = false
"#;

fn load(ops_toml: &str) -> anyhow::Result<Config> {
    let dir = tempfile::tempdir().unwrap();
    let root = crate::test_utils::canonical_root(&dir);
    let _xdg = crate::test_utils::isolate_global_config(&root);
    std::fs::write(root.join(".ops.toml"), ops_toml).unwrap();
    load_config_at(&root)
}

fn exec<'a>(config: &'a Config, name: &str) -> &'a ExecCommandSpec {
    match config.commands.get(name) {
        Some(CommandSpec::Exec(e)) => e,
        other => panic!("{name} must be an exec command, got {other:?}"),
    }
}

fn cell_args(spec: &ExecCommandSpec, name: &str) -> Vec<String> {
    spec.matrix_cells(name)
        .expect("cells")
        .iter()
        .map(|c| format!("{} => {}", c.id, c.spec.args.join(" ")))
        .collect()
}

fn load_err(ops_toml: &str) -> String {
    format!(
        "{:#}",
        load(ops_toml).expect_err("config must fail to load")
    )
}

#[test]
#[serial_test::serial]
fn matrix_command_expands_one_spec_per_cell() {
    let config = load(DOC_DEFAULT).expect("matrix config must load");
    let spec = exec(&config, "doc-default");
    let strategy = spec.strategy.as_ref().expect("strategy parsed");
    assert_eq!(strategy.max_parallel, Some(1));
    assert!(!strategy.fail_fast);

    let cells = spec.matrix_cells("doc-default").unwrap();
    assert_eq!(
        cell_args(spec, "doc-default"),
        [
            "doc-default [crate=dbsec-core] => doc --no-deps -p dbsec-core",
            "doc-default [crate=dbsec] => doc --no-deps -p dbsec",
            "doc-default [crate=dbsec-vault] => doc --no-deps -p dbsec-vault",
        ]
    );
    // `${VAR}` is left for the runner's expansion pass; each cell has no
    // strategy of its own.
    assert_eq!(
        cells[0].spec.env["CARGO_TARGET_DIR"],
        "${CARGO_TARGET_DIR:-target}/doc-default"
    );
    assert!(cells.iter().all(|c| c.spec.strategy.is_none()));
}

#[test]
#[serial_test::serial]
fn matrix_is_substituted_in_env_values_and_cwd() {
    let config = load(
        r#"
[commands.t]
program = "make"
cwd = "crates/${matrix.crate}"
env = { FEATURES = "${matrix.features}" }
[commands.t.strategy]
matrix = { crate = ["a"], features = ["x", "y"] }
"#,
    )
    .unwrap();
    let cells = exec(&config, "t").matrix_cells("t").unwrap();
    let rendered: Vec<(String, String, String)> = cells
        .iter()
        .map(|c| {
            (
                c.id.clone(),
                c.spec.env["FEATURES"].clone(),
                c.spec.cwd.as_ref().unwrap().display().to_string(),
            )
        })
        .collect();
    assert_eq!(
        rendered,
        [
            (
                "t [crate=a, features=x]".into(),
                "x".into(),
                "crates/a".into()
            ),
            (
                "t [crate=a, features=y]".into(),
                "y".into(),
                "crates/a".into()
            ),
        ]
    );
}

#[test]
#[serial_test::serial]
fn unknown_matrix_key_is_a_load_error_naming_the_command() {
    let msg = load_err(&DOC_DEFAULT.replace("${matrix.crate}", "${matrix.crat}"));
    assert!(msg.contains("'doc-default'"), "{msg}");
    assert!(msg.contains("${matrix.crat}"), "{msg}");
    assert!(msg.contains("args[3]"), "{msg}");
}

/// A reference no cell of an include-only key defines fails too — every
/// cell must resolve every reference.
#[test]
#[serial_test::serial]
fn key_missing_from_some_cell_is_a_load_error() {
    let msg = load_err(
        r#"
[commands.t]
program = "echo"
args = ["${matrix.crate}", "${matrix.extra}"]
[commands.t.strategy]
matrix = { crate = ["a", "b"], include = [{ crate = "a", extra = "1" }] }
"#,
    );
    assert!(msg.contains("[crate=b]"), "{msg}");
    assert!(msg.contains("${matrix.extra}"), "{msg}");
}

#[test]
#[serial_test::serial]
fn matrix_reference_without_a_strategy_is_a_load_error() {
    let msg = load_err(
        r#"
[commands.t]
program = "echo"
env = { X = "${matrix.crate}" }
"#,
    );
    assert!(msg.contains("'t'"), "{msg}");
    assert!(msg.contains("env[X]"), "{msg}");
    assert!(msg.contains("no [commands.t.strategy]"), "{msg}");
}

#[test]
#[serial_test::serial]
fn matrix_reference_in_program_is_a_load_error() {
    let msg = load_err(
        r#"
[commands.t]
program = "${matrix.tool}"
[commands.t.strategy]
matrix = { tool = ["a"] }
"#,
    );
    assert!(msg.contains("program"), "{msg}");
}

#[test]
#[serial_test::serial]
fn zero_max_parallel_and_malformed_matrices_fail_the_load() {
    let msg = load_err(&DOC_DEFAULT.replace("max_parallel = 1", "max_parallel = 0"));
    assert!(msg.contains("max_parallel"), "{msg}");
    let msg = load_err(&DOC_DEFAULT.replace(r#"["dbsec-core", "dbsec", "dbsec-vault"]"#, "[]"));
    assert!(msg.contains("no values"), "{msg}");
}

#[test]
#[serial_test::serial]
fn strategy_on_a_composite_is_rejected() {
    let msg = load_err(
        r#"
[commands.g]
commands = ["a"]
[commands.g.strategy]
matrix = { k = ["a"] }
"#,
    );
    assert!(msg.contains("strategy"), "{msg}");
}

/// AC #7: a clone copies the strategy.
#[test]
#[serial_test::serial]
fn clone_copies_the_strategy() {
    let config = load(&format!(
        "{DOC_DEFAULT}\n[commands.doc-copy]\nclone = \"doc-default\"\n"
    ))
    .unwrap();
    let copy = exec(&config, "doc-copy");
    assert_eq!(copy.strategy, exec(&config, "doc-default").strategy);
    assert_eq!(
        copy.matrix_cells("doc-copy").unwrap()[0].id,
        "doc-copy [crate=dbsec-core]",
        "cells are labelled by the clone's own name"
    );
}

/// AC #7: a strategy beside `clone` replaces the copied one wholesale.
#[test]
#[serial_test::serial]
fn strategy_beside_clone_replaces_the_copy() {
    let config = load(&format!(
        "{DOC_DEFAULT}\n[commands.doc-copy]\nclone = \"doc-default\"\n\
         [commands.doc-copy.strategy]\nmatrix = {{ crate = [\"other\"] }}\n"
    ))
    .unwrap();
    let strategy = exec(&config, "doc-copy").strategy.as_ref().unwrap();
    assert_eq!(strategy.matrix.axes["crate"], ["other"]);
    assert_eq!(strategy.max_parallel, None, "no field of the copy survives");
    assert!(strategy.fail_fast, "the replacement's own default applies");
}

#[test]
#[serial_test::serial]
fn strategy_beside_a_composite_clone_is_rejected() {
    let msg = load_err(
        "[commands.a]\nprogram = \"echo\"\n[commands.g]\ncommands = [\"a\"]\n\
         [commands.g2]\nclone = \"g\"\n[commands.g2.strategy]\nmatrix = { k = [\"a\"] }\n",
    );
    assert!(msg.contains("`strategy`"), "{msg}");
}

/// AC #7: `[extend.<name>] matrix.<key>` appends values to that axis, and
/// composes with a clone (`[extend.<clone>]` grows only the copy).
#[test]
#[serial_test::serial]
fn extend_matrix_appends_values_to_an_axis() {
    let config = load(&format!(
        "{DOC_DEFAULT}\n[commands.doc-copy]\nclone = \"doc-default\"\n\
         [extend.doc-copy]\nmatrix.crate = [\"dbsec-derive\"]\n"
    ))
    .unwrap();
    assert_eq!(
        exec(&config, "doc-copy")
            .strategy
            .as_ref()
            .unwrap()
            .matrix
            .axes["crate"],
        ["dbsec-core", "dbsec", "dbsec-vault", "dbsec-derive"]
    );
    assert_eq!(
        exec(&config, "doc-default")
            .strategy
            .as_ref()
            .unwrap()
            .matrix
            .axes["crate"]
            .len(),
        3,
        "the clone's source is not extended"
    );
}

#[test]
#[serial_test::serial]
fn extend_matrix_with_an_unknown_axis_or_no_strategy_fails() {
    let msg = load_err(&format!(
        "{DOC_DEFAULT}\n[extend.doc-default]\nmatrix.crat = [\"x\"]\n"
    ));
    assert!(msg.contains("matrix.crat"), "{msg}");
    let msg = load_err("[commands.t]\nprogram = \"echo\"\n[extend.t]\nmatrix.k = [\"x\"]\n");
    assert!(msg.contains("no [commands.t.strategy]"), "{msg}");
}
