//! Tests for expand_to_leaves and alias resolution.

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

/// TQ-012: Tests for depth limit in expand_to_leaves.
mod depth_limit_tests {
    use super::*;

    fn create_nested_commands(depth: usize) -> HashMap<String, CommandSpec> {
        let mut commands = HashMap::new();
        for i in 0..depth {
            let name = format!("level_{}", i);
            let next_name = format!("level_{}", i + 1);
            commands.insert(
                name,
                CommandSpec::Composite(ops_core::config::CompositeCommandSpec::new([next_name])),
            );
        }
        commands.insert(
            format!("level_{}", depth),
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
    /// them into one pass via `canonical_with_spec` cuts roughly half the
    /// lookups on every visit. This microbench builds a representative
    /// composite graph (one root that fans out 50 wide → 5 mid composites
    /// → 10 leaves each) and asserts that 1k expansions complete within a
    /// generous wall-clock budget on the slowest CI runners. The point is
    /// not to be a regression detector with millisecond precision but to
    /// guard against an order-of-magnitude regression sneaking back in.
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
        let start = std::time::Instant::now();
        for _ in 0..1_000 {
            let plan = runner.expand_to_leaves("root").expect("expand");
            assert_eq!(plan.len(), 50 * 10);
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed < std::time::Duration::from_secs(2),
            "expand_to_leaves microbench regressed: 1k expansions took {elapsed:?}"
        );
    }
}
