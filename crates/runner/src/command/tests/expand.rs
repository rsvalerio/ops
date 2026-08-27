//! Tests for `expand_to_leaves` and alias resolution.

use super::*;

#[test]
fn expand_to_leaves_single() {
    let runner = runner_with_test_commands();
    let plan = runner
        .expand_to_leaves("build")
        .expect("build must exist in test config");
    assert_eq!(plan, vec!["build"]);
}

#[test]
fn expand_to_leaves_composite() {
    let runner = runner_with_test_commands();
    let plan = runner
        .expand_to_leaves("verify")
        .expect("verify must exist in test config");
    assert_eq!(plan, vec!["build", "clippy"]);
}

#[test]
fn expand_to_leaves_unknown() {
    let runner = runner_with_test_commands();
    assert!(matches!(
        runner.expand_to_leaves("unknown"),
        Err(ExpandError::Unknown(_))
    ));
}

#[test]
fn resolve_by_alias() {
    let mut commands = HashMap::new();
    let mut spec = exec_spec("cargo", &["build"]);
    spec.aliases = vec!["b".to_string(), "compile".to_string()];
    commands.insert("build".to_string(), CommandSpec::Exec(spec));
    let runner = test_runner(commands);

    assert!(runner.resolve("build").is_some());
    assert!(runner.resolve("b").is_some());
    assert!(runner.resolve("compile").is_some());
    assert!(runner.resolve("unknown").is_none());
}

/// PERF-3 / TASK-0774: `register_commands` merges new aliases incrementally
/// rather than rebuilding the full alias map on each batch. Verify that:
/// - aliases registered across N successive 1-entry batches are all resolvable
/// - re-registering the same id with different aliases prunes the stale ones
/// - cross-extension alias collisions still surface as before
#[test]
fn register_commands_incremental_alias_merge_preserves_resolution() {
    let mut runner = test_runner(HashMap::new());
    for n in 0..5 {
        let id = format!("cmd_{n}");
        let alias = format!("c{n}");
        let mut spec = exec_spec("echo", &[id.as_str()]);
        spec.aliases = vec![alias.clone()];
        runner.register_commands(vec![(id.clone().into(), CommandSpec::Exec(spec))]);
    }
    for n in 0..5 {
        let alias = format!("c{n}");
        let canonical = format!("cmd_{n}");
        assert!(
            runner.resolve(&alias).is_some(),
            "alias `{alias}` registered in an earlier batch must remain resolvable"
        );
        assert!(
            runner.resolve(&canonical).is_some(),
            "canonical id `{canonical}` must remain resolvable"
        );
    }
}

#[test]
fn register_commands_re_registration_prunes_stale_aliases() {
    let mut runner = test_runner(HashMap::new());

    let mut first = exec_spec("echo", &["v1"]);
    first.aliases = vec!["alias_a".to_string()];
    runner.register_commands(vec![("cmd".into(), CommandSpec::Exec(first))]);
    assert!(runner.resolve("alias_a").is_some());

    let mut second = exec_spec("echo", &["v2"]);
    second.aliases = vec!["alias_b".to_string()];
    runner.register_commands(vec![("cmd".into(), CommandSpec::Exec(second))]);

    assert!(
        runner.resolve("alias_b").is_some(),
        "new alias on re-registration must resolve"
    );
    assert!(
        runner.resolve("alias_a").is_none(),
        "old alias dropped from the new spec must no longer point at the (now-replaced) command"
    );
}

/// CONC-3 / TASK-1137: when an extension/stack alias collides with a
/// config-defined alias of the same name, `merge_alias_for` must surface
/// the cross-store collision via `tracing::warn!`. Pre-fix only same-store
/// (stack vs extension) collisions warned, so a config alias silently
/// shadowed an extension alias at lookup time with no audit trail.
#[test]
fn register_commands_warns_on_cross_store_alias_collision_with_config() {
    use std::io::Write;
    use std::sync::{Arc as StdArc, Mutex as StdMutex};

    #[derive(Clone)]
    struct VecWriter(StdArc<StdMutex<Vec<u8>>>);
    impl Write for VecWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for VecWriter {
        type Writer = Self;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    // Seed a config that defines `cfg_cmd` with alias `shared`.
    let mut commands = HashMap::new();
    let mut cfg_spec = exec_spec("echo", &["cfg"]);
    cfg_spec.aliases = vec!["shared".to_string()];
    commands.insert("cfg_cmd".to_string(), CommandSpec::Exec(cfg_spec));
    let mut runner = test_runner(commands);

    let buf = StdArc::new(StdMutex::new(Vec::<u8>::new()));
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_writer(VecWriter(StdArc::clone(&buf)))
        .with_ansi(false)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        // Register an extension command that re-uses `shared` as its alias.
        let mut ext_spec = exec_spec("echo", &["ext"]);
        ext_spec.aliases = vec!["shared".to_string()];
        runner.register_commands(vec![("ext_cmd".into(), CommandSpec::Exec(ext_spec))]);
    });

    let logged = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("alias collision")
            && logged.contains("cfg_cmd")
            && logged.contains("ext_cmd"),
        "warn must name both owners on cross-store config-vs-extension collision; got: {logged}"
    );
}

#[test]
fn expand_to_leaves_via_alias() {
    let mut commands = HashMap::new();
    let mut spec = exec_spec("cargo", &["build"]);
    spec.aliases = vec!["b".to_string()];
    commands.insert("build".to_string(), CommandSpec::Exec(spec));
    let runner = test_runner(commands);

    let plan = runner.expand_to_leaves("b").expect("alias must resolve");
    assert_eq!(plan, vec!["build"]);
}

/// ERR-1 / TASK-1089: when a configured command that owns an alias is
/// deleted (a user edits `.ops.toml` and removes the entry), the alias
/// must still resolve if the same canonical name exists as a stack
/// default. The pre-fix `resolve_alias` returned the `None` from
/// `config.commands.get(name)` directly without consulting the stack /
/// extension stores; the fix falls through to those stores so the alias
/// stays usable.
///
/// The test seeds a config with a command + alias, deletes the command,
/// and verifies the alias resolves to a stack default of the same name.
/// It also exercises the orphan-fallthrough branch added to
/// `resolve_alias` and `canonical_with_spec` by repopulating the
/// non-config alias map after deletion (matching what
/// `register_commands` would do for an extension that owns the alias).
#[test]
fn orphan_config_alias_falls_through_to_stack_default() {
    use ops_core::config::CommandId;

    // Seed: config command "build" with alias "b".
    let mut commands = HashMap::new();
    let mut spec = exec_spec("cargo", &["build"]);
    spec.aliases = vec!["b".to_string()];
    commands.insert("build".to_string(), CommandSpec::Exec(spec));
    let mut runner = test_runner(commands);

    // Stack default of the same canonical name. Inserted post-construction
    // because `test_runner` does not detect a stack from a synthetic
    // working directory.
    runner.stack_commands.insert(
        CommandId::from("build"),
        CommandSpec::Exec(exec_spec("echo", &["from-stack"])),
    );

    // Sanity: with the config entry present the alias resolves to the
    // config spec.
    assert!(runner.resolve("b").is_some());

    // Simulate the config edit that removed the underlying command.
    // ARCH-9 / TASK-0993: the runner's persistent `data_context` also holds
    // an `Arc<Config>` clone (single-source-of-truth for the data cache),
    // so this test no longer has the runner as the unique Config owner.
    // `Arc::make_mut` clones the Config when shared and rebinds runner.config
    // to the new Arc; the data_context's stale clone is irrelevant for this
    // alias-resolution test.
    {
        let cfg = Arc::make_mut(&mut runner.config);
        cfg.commands.shift_remove("build");
    }
    // Mirror the alias on the non-config alias map (what an extension or
    // stack default that re-declared the alias would produce); without
    // this entry there is nothing to resolve "b" to in any store.
    runner
        .non_config_alias_map
        .insert("b".to_string(), "build".to_string());

    // The alias must still resolve — to the stack default — rather than
    // returning `None` (the pre-fix bug).
    assert!(
        runner.resolve("b").is_some(),
        "alias `b` must fall through to the stack default after the config command was deleted"
    );
    assert!(
        runner.resolve("build").is_some(),
        "canonical name resolves via the stack store"
    );

    // expand_to_leaves drives `canonical_with_spec`; the single-pass
    // canonical+spec lookup must agree with the double-pass `resolve`
    // path (AC #3).
    let plan = runner
        .expand_to_leaves("b")
        .expect("alias must canonicalize via canonical_with_spec");
    assert_eq!(plan, vec!["build"]);
}

/// Built-in CLI subcommands (`end-of-file-fixer`, `trailing-whitespace`) and
/// their visible aliases (`eof`, `tw`) must resolve through the composite
/// resolver so users can list them in `commands = [...]`. Before the
/// `builtin_commands` store was added the resolver only walked
/// config / stack / extension, producing `unknown command: eof` for a
/// run-before-commit composite that referenced the alias.
#[test]
fn builtin_commands_resolve_by_name_and_alias() {
    let runner = test_runner(HashMap::new());
    assert!(
        runner.resolve("end-of-file-fixer").is_some(),
        "canonical builtin name must resolve"
    );
    assert!(runner.resolve("eof").is_some(), "alias must resolve");
    assert!(
        runner.resolve("trailing-whitespace").is_some(),
        "canonical builtin name must resolve"
    );
    assert!(runner.resolve("tw").is_some(), "alias must resolve");
}

#[test]
fn composite_can_reference_builtin_aliases() {
    let mut commands = HashMap::new();
    commands.insert(
        "pre-commit".to_string(),
        CommandSpec::Composite(composite_cmd(&["eof", "tw"])),
    );
    let runner = test_runner(commands);
    let plan = runner
        .expand_to_leaves("pre-commit")
        .expect("composite referencing builtin aliases must expand");
    assert_eq!(plan, vec!["end-of-file-fixer", "trailing-whitespace"]);
}

mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn expand_to_leaves_single_exec_returns_self(id in "[a-zA-Z_][a-zA-Z0-9_]{0,10}") {
            let mut commands = HashMap::new();
            commands.insert(
                id.clone(),
                CommandSpec::Exec(exec_spec("cargo", &["build"])),
            );
            let runner = test_runner(commands);
            let result = runner.expand_to_leaves(&id);
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap(), vec![id]);
        }

        #[test]
        fn expand_to_leaves_composite_flattens(
            name in "grp[a-zA-Z0-9_]{0,5}",
            cmd1 in "a[a-zA-Z0-9_]{0,5}",
            cmd2 in "b[a-zA-Z0-9_]{0,5}"
        ) {
            let mut commands = HashMap::new();
            commands.insert(cmd1.clone(), CommandSpec::Exec(exec_spec("echo", &[&cmd1])));
            commands.insert(cmd2.clone(), CommandSpec::Exec(exec_spec("echo", &[&cmd2])));
            commands.insert(
                name.clone(),
                CommandSpec::Composite(ops_core::config::CompositeCommandSpec::new([
                    cmd1.clone(),
                    cmd2.clone(),
                ])),
            );
            let runner = test_runner(commands);
            let result = runner.expand_to_leaves(&name);
            prop_assert!(result.is_ok());
            let leaves = result.unwrap();
            prop_assert!(leaves.iter().any(|l| l == cmd1.as_str()));
            prop_assert!(leaves.iter().any(|l| l == cmd2.as_str()));
            prop_assert!(!leaves.iter().any(|l| l == name.as_str()));
        }

        #[test]
        fn expand_to_leaves_unknown_returns_none(id in "unknown[a-zA-Z0-9_]{0,8}") {
            let runner = test_runner(HashMap::new());
            let result = runner.expand_to_leaves(&id);
            prop_assert!(matches!(result, Err(ExpandError::Unknown(_))));
        }
    }
}

/// TASK-1657: composite trees must agree with themselves on the scheduling
/// flags, because the expanded plan is flat and scheduled as a single unit.
///
/// Before this, `expand_inner` OR-folded `parallel` and `fail_fast` across the
/// whole traversal, so one `parallel = true` descendant silently promoted a
/// `parallel = false` ancestor and the config read as though it were
/// sequential while every step ran concurrently. Option 3 of TASK-1657
/// (reject at validation) keeps the flat plan model and makes the trap loud.
mod schedule_flag_agreement_tests {
    use super::*;

    /// Build a runner with `outer -> inner -> [a, b]`, letting each composite's
    /// `(parallel, fail_fast)` be set independently.
    fn nested_runner(outer: (bool, bool), inner: (bool, bool)) -> crate::command::CommandRunner {
        let mut commands = HashMap::new();
        commands.insert("a".to_string(), CommandSpec::Exec(echo_cmd("a")));
        commands.insert("b".to_string(), CommandSpec::Exec(echo_cmd("b")));

        let mut inner_spec = composite_cmd(&["a", "b"]);
        inner_spec.parallel = inner.0;
        inner_spec.fail_fast = inner.1;
        commands.insert("inner".to_string(), CommandSpec::Composite(inner_spec));

        let mut outer_spec = composite_cmd(&["inner"]);
        outer_spec.parallel = outer.0;
        outer_spec.fail_fast = outer.1;
        commands.insert("outer".to_string(), CommandSpec::Composite(outer_spec));

        test_runner(commands)
    }

    /// The headline case from TASK-1656: sequential parent, parallel child.
    #[test]
    fn parallel_child_under_sequential_parent_is_rejected() {
        let runner = nested_runner((false, true), (true, true));
        let err = runner
            .expand_to_leaves("outer")
            .expect_err("parallel child under sequential parent must be rejected");
        assert!(
            matches!(
                &err,
                ExpandError::ConflictingSchedule {
                    flag: "parallel",
                    root,
                    root_value: false,
                    conflicting,
                    conflicting_value: true,
                } if root == "outer" && conflicting == "inner"
            ),
            "unexpected error: {err:?}"
        );
    }

    /// The reverse direction is equally a lie under a flat plan: `inner`
    /// declares `parallel = false` but every step would run concurrently.
    #[test]
    fn sequential_child_under_parallel_parent_is_rejected() {
        let runner = nested_runner((true, true), (false, true));
        let err = runner
            .expand_to_leaves("outer")
            .expect_err("sequential child under parallel parent must be rejected");
        assert!(
            matches!(
                &err,
                ExpandError::ConflictingSchedule {
                    flag: "parallel",
                    root_value: true,
                    conflicting_value: false,
                    ..
                }
            ),
            "unexpected error: {err:?}"
        );
    }

    /// `fail_fast` gets the same treatment: a `false` descendant must not
    /// silently disable fail-fast for a plan whose root enables it.
    #[test]
    fn fail_fast_disagreement_is_rejected() {
        let runner = nested_runner((false, true), (false, false));
        let err = runner
            .expand_to_leaves("outer")
            .expect_err("fail_fast disagreement must be rejected");
        assert!(
            matches!(
                &err,
                ExpandError::ConflictingSchedule {
                    flag: "fail_fast",
                    root_value: true,
                    conflicting_value: false,
                    ..
                }
            ),
            "unexpected error: {err:?}"
        );
    }

    /// The check rejects *disagreement*, not nesting. Agreeing trees expand
    /// exactly as before, in both the all-sequential and all-parallel shapes.
    #[test]
    fn agreeing_nested_composites_still_expand() {
        for flags in [(false, true), (true, true), (true, false), (false, false)] {
            let runner = nested_runner(flags, flags);
            let plan = runner
                .expand_to_leaves("outer")
                .unwrap_or_else(|e| panic!("agreeing tree {flags:?} must expand, got: {e:?}"));
            assert_eq!(plan, vec!["a", "b"], "flags {flags:?}");
        }
    }

    /// The aggregated flags returned alongside the leaves must reflect the
    /// (now uniform) declared value, so callers scheduling the plan agree with
    /// what the config says.
    #[test]
    fn aggregated_flags_match_the_agreed_declaration() {
        let runner = nested_runner((true, false), (true, false));
        let (leaves, any_parallel, fail_fast_disabled) = runner
            .expand_to_leaves_with_flags("outer")
            .expect("agreeing tree must expand");
        assert_eq!(leaves, vec!["a", "b"]);
        assert!(any_parallel);
        assert!(fail_fast_disabled);
    }

    /// A diamond (`root -> [x, y]`, both `-> shared`) revisits `shared`. The
    /// agreement check must not mistake a second visit to an *agreeing* node
    /// for a conflict — the cycle guard already tolerates this shape.
    #[test]
    fn diamond_revisit_of_agreeing_node_is_not_a_conflict() {
        let mut commands = HashMap::new();
        commands.insert("leaf".to_string(), CommandSpec::Exec(echo_cmd("l")));
        commands.insert(
            "shared".to_string(),
            CommandSpec::Composite(composite_cmd(&["leaf"])),
        );
        commands.insert(
            "x".to_string(),
            CommandSpec::Composite(composite_cmd(&["shared"])),
        );
        commands.insert(
            "y".to_string(),
            CommandSpec::Composite(composite_cmd(&["shared"])),
        );
        commands.insert(
            "root".to_string(),
            CommandSpec::Composite(composite_cmd(&["x", "y"])),
        );

        let runner = test_runner(commands);
        let plan = runner
            .expand_to_leaves("root")
            .expect("diamond must expand");
        assert_eq!(plan, vec!["leaf", "leaf"]);
    }

    /// The message is the whole point of choosing rejection over a silent
    /// winner, so pin that it names both composites, the flag, both values,
    /// and offers a concrete fix.
    #[test]
    fn conflict_message_is_actionable() {
        let runner = nested_runner((false, true), (true, true));
        let msg = runner
            .expand_to_leaves("outer")
            .expect_err("must conflict")
            .to_string();
        for expected in [
            "conflicting `parallel`",
            "`outer`",
            "`inner`",
            "parallel = false",
            "parallel = true",
            "fix:",
        ] {
            assert!(
                msg.contains(expected),
                "error message missing {expected:?}, got:\n{msg}"
            );
        }
    }
}

mod nested_composite_tests {
    use super::*;

    #[test]
    fn expand_to_leaves_deeply_nested_composite() {
        let mut commands = HashMap::new();
        commands.insert("leaf1".to_string(), CommandSpec::Exec(echo_cmd("1")));
        commands.insert("leaf2".to_string(), CommandSpec::Exec(echo_cmd("2")));
        commands.insert("leaf3".to_string(), CommandSpec::Exec(echo_cmd("3")));

        commands.insert(
            "level2_a".to_string(),
            CommandSpec::Composite(composite_cmd(&["leaf1", "leaf2"])),
        );
        commands.insert(
            "level2_b".to_string(),
            CommandSpec::Composite(composite_cmd(&["leaf3"])),
        );
        commands.insert(
            "level3".to_string(),
            CommandSpec::Composite(composite_cmd(&["level2_a", "level2_b"])),
        );

        let runner = test_runner(commands);
        let plan = runner.expand_to_leaves("level3").expect("should resolve");
        assert_eq!(plan, vec!["leaf1", "leaf2", "leaf3"]);
    }

    #[test]
    fn expand_to_leaves_nested_missing_intermediate() {
        let mut commands = HashMap::new();
        commands.insert("leaf".to_string(), CommandSpec::Exec(echo_cmd("1")));
        commands.insert(
            "level2".to_string(),
            CommandSpec::Composite(composite_cmd(&["nonexistent"])),
        );
        commands.insert(
            "level3".to_string(),
            CommandSpec::Composite(composite_cmd(&["level2"])),
        );

        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("level3").is_err(),
            "missing intermediate command should return None"
        );
    }

    /// PATTERN-1 / TASK-0505: a diamond composite topology (two siblings
    /// referencing the same composite child) is a DAG, not a cycle. The
    /// previous "all-time visited" set incorrectly flagged the second visit
    /// to D as a cycle; the fix tracks only the active recursion stack.
    #[test]
    fn expand_to_leaves_diamond_composite_succeeds() {
        let mut commands = HashMap::new();
        commands.insert("d_leaf".to_string(), CommandSpec::Exec(echo_cmd("d")));
        // D = composite that wraps a single leaf so it is a composite node
        // visited from both branches.
        commands.insert(
            "D".to_string(),
            CommandSpec::Composite(composite_cmd(&["d_leaf"])),
        );
        commands.insert(
            "B".to_string(),
            CommandSpec::Composite(composite_cmd(&["D"])),
        );
        commands.insert(
            "C".to_string(),
            CommandSpec::Composite(composite_cmd(&["D"])),
        );
        commands.insert(
            "A".to_string(),
            CommandSpec::Composite(composite_cmd(&["B", "C"])),
        );

        let runner = test_runner(commands);
        let plan = runner
            .expand_to_leaves("A")
            .expect("diamond DAG must expand without a false-positive cycle error");
        assert_eq!(plan, vec!["d_leaf", "d_leaf"]);
    }

    #[test]
    fn expand_to_leaves_deep_cycle() {
        let mut commands = HashMap::new();
        commands.insert("leaf".to_string(), CommandSpec::Exec(echo_cmd("1")));
        commands.insert(
            "level2".to_string(),
            CommandSpec::Composite(composite_cmd(&["level3"])),
        );
        commands.insert(
            "level3".to_string(),
            CommandSpec::Composite(composite_cmd(&["level2"])),
        );

        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("level2").is_err(),
            "deep cycle should return None"
        );
    }
}

mod cycle_detection_tests {
    use super::*;

    #[test]
    fn expand_to_leaves_cycle_2_nodes() {
        let mut commands = HashMap::new();
        commands.insert(
            "a".to_string(),
            CommandSpec::Composite(composite_cmd(&["b"])),
        );
        commands.insert(
            "b".to_string(),
            CommandSpec::Composite(composite_cmd(&["a"])),
        );
        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("a").is_err(),
            "2-node cycle should return None"
        );
    }

    #[test]
    fn expand_to_leaves_cycle_3_nodes() {
        let mut commands = HashMap::new();
        commands.insert(
            "a".to_string(),
            CommandSpec::Composite(composite_cmd(&["b"])),
        );
        commands.insert(
            "b".to_string(),
            CommandSpec::Composite(composite_cmd(&["c"])),
        );
        commands.insert(
            "c".to_string(),
            CommandSpec::Composite(composite_cmd(&["a"])),
        );
        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("a").is_err(),
            "3-node cycle a->b->c->a should return None"
        );
    }

    #[test]
    fn expand_to_leaves_self_reference() {
        let mut commands = HashMap::new();
        commands.insert(
            "self_ref".to_string(),
            CommandSpec::Composite(composite_cmd(&["self_ref"])),
        );
        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("self_ref").is_err(),
            "self-referencing command should return None"
        );
    }
}

/// TQ-012: Tests for depth limit in `expand_to_leaves`.
mod depth_limit_tests {
    use super::*;

    fn create_nested_commands(depth: usize) -> HashMap<String, CommandSpec> {
        let mut commands = HashMap::new();
        for i in 0..depth {
            let name = format!("level_{i}");
            // `i < depth`, so `i + 1 <= depth` and this is exactly `+ 1`.
            let next_name = format!("level_{}", i.saturating_add(1));
            commands.insert(
                name,
                CommandSpec::Composite(ops_core::config::CompositeCommandSpec::new([next_name])),
            );
        }
        commands.insert(
            format!("level_{depth}"),
            CommandSpec::Exec(exec_spec("echo", &["leaf"])),
        );
        commands
    }

    #[test]
    fn expand_to_leaves_shallow_nesting_succeeds() {
        let commands = create_nested_commands(10);
        let runner = test_runner(commands);
        let result = runner.expand_to_leaves("level_0");
        assert!(result.is_ok(), "10 levels should be well within limit");
    }

    #[test]
    fn expand_to_leaves_at_depth_limit_succeeds() {
        let commands = create_nested_commands(99);
        let runner = test_runner(commands);
        let result = runner.expand_to_leaves("level_0");
        assert!(
            result.is_ok(),
            "99 levels (depth=99 starting from 0) should succeed at MAX_DEPTH=100"
        );
    }

    #[test]
    fn expand_to_leaves_exceeds_depth_limit_returns_none() {
        let commands = create_nested_commands(101);
        let runner = test_runner(commands);
        let result = runner.expand_to_leaves("level_0");
        assert!(
            matches!(result, Err(ExpandError::DepthExceeded { .. })),
            "101 levels (exceeds MAX_DEPTH=100) should return DepthExceeded"
        );
    }

    /// PERF-3 / TASK-0766: pin the post-fold hot path. The pre-fix code paid
    /// two store traversals per node (`canonical_id` + `resolve`); folding
    /// them into one pass via `canonical_with_spec` cuts the lookups on every
    /// visit in half.
    ///
    /// TEST-15 / TASK-1664: asserted by **counting traversals**, not by
    /// timing. The previous form ran 1k expansions against a two-second
    /// wall-clock budget, which failed at 9.8s under CPU contention — the
    /// normal state of a shared CI runner, and the reason this test could not
    /// be enabled there. The budget was also too coarse to catch what it
    /// guarded: a 2x regression would not reliably breach two seconds, so it
    /// only ever caught catastrophic slowdowns.
    ///
    /// The count is exact. Expanding the graph below visits 551 nodes
    /// (1 root + 50 mids + 500 leaves), so a correct single-pass expansion
    /// performs exactly 551 store walks. Reverting to `canonical_id` +
    /// `resolve` would report 1102 and fail immediately — precisely the
    /// regression the wall-clock version was blind to.
    #[test]
    fn expand_to_leaves_microbench_does_not_regress() {
        let mut commands = HashMap::new();
        for leaf in 0..10 {
            commands.insert(
                format!("leaf_{leaf}"),
                CommandSpec::Exec(echo_cmd(&format!("{leaf}"))),
            );
        }
        let leaf_names: Vec<String> = (0..10).map(|i| format!("leaf_{i}")).collect();
        let leaf_refs: Vec<&str> = leaf_names.iter().map(String::as_str).collect();
        for mid in 0..50 {
            commands.insert(
                format!("mid_{mid}"),
                CommandSpec::Composite(composite_cmd(&leaf_refs)),
            );
        }
        let mid_names: Vec<String> = (0..50).map(|i| format!("mid_{i}")).collect();
        let mid_refs: Vec<&str> = mid_names.iter().map(String::as_str).collect();
        commands.insert(
            "root".to_string(),
            CommandSpec::Composite(composite_cmd(&mid_refs)),
        );

        let runner = test_runner(commands);

        // One expansion, counted exactly. The counter is process-global and
        // other tests in this binary also walk the stores, so measure a delta
        // around a single call rather than an absolute total.
        let before = crate::command::resolve::store_walk_count();
        let plan = runner.expand_to_leaves("root").expect("expand");
        let walks = crate::command::resolve::store_walk_count() - before;

        assert_eq!(plan.len(), 50 * 10);

        // 1 root + 50 mid composites + 500 leaf visits.
        let nodes = 1 + 50 + (50 * 10);
        assert_eq!(
            walks, nodes,
            "expand_to_leaves must walk the command stores exactly once per \
             visited node ({nodes}); saw {walks}. Twice that means the \
             canonical_id + resolve double-lookup came back (TASK-0766)."
        );
    }
}
