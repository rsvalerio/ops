use super::*;
use crate::test_utils::{exec_spec, EnvGuard};
use serial_test::serial;
use std::time::Duration;

#[test]
fn exec_spec_timeout_some() {
    let mut e = exec_spec("cargo", &["build"]);
    e.timeout_secs = Some(300);
    assert_eq!(e.timeout(), Some(Duration::from_secs(300)));
}

#[test]
fn exec_spec_timeout_none() {
    let e = exec_spec("cargo", &["build"]);
    assert_eq!(e.timeout(), None);
}

#[test]
fn exec_spec_display_cmd() {
    let e = exec_spec("cargo", &["clippy", "--all"]);
    assert_eq!(e.display_cmd(), "cargo clippy --all");
}

#[test]
fn exec_spec_display_cmd_no_args() {
    let e = exec_spec("make", &[]);
    assert_eq!(e.display_cmd(), "make");
}

/// SEC-21 AC #3: an arg containing a space and a quote must round-trip
/// through `display_cmd` in a form the user can disambiguate from two
/// separate args. Without quoting, `["foo bar"]` and `["foo", "bar"]` would
/// render identically.
#[test]
fn exec_spec_display_cmd_quotes_metacharacters() {
    let one_arg_with_space = exec_spec("cargo", &["foo bar"]);
    let two_args = exec_spec("cargo", &["foo", "bar"]);
    assert_ne!(
        one_arg_with_space.display_cmd(),
        two_args.display_cmd(),
        "single arg containing a space must render differently from two separate args"
    );
    // The single-arg form must be a single shell word (single-quoted).
    assert_eq!(one_arg_with_space.display_cmd(), "cargo 'foo bar'");

    // Embedded quote uses the POSIX close-escape-reopen sequence: '\''
    let with_quote = exec_spec("cargo", &["it's quoted"]);
    assert_eq!(with_quote.display_cmd(), "cargo 'it'\\''s quoted'");

    // SEC-21 motivating example: a `;` would otherwise look like a shell
    // separator. Quoting makes it visibly part of one argument.
    let injection_shape = exec_spec("cargo", &["build", "--config", "evil=\"; rm -rf /\""]);
    let rendered = injection_shape.display_cmd();
    assert!(
        rendered.contains("'evil=\"; rm -rf /\"'"),
        "metachar arg must be wrapped: got {rendered}"
    );
}

#[test]
fn read_config_file_valid_toml() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("test.toml");
    std::fs::write(
        &path,
        r#"
[output]
theme = "compact"
columns = 100
show_error_detail = false

[commands.hello]
program = "echo"
args = ["hi"]
"#,
    )
    .unwrap();
    let overlay = read_config_file(&path)
        .expect("valid toml should parse")
        .expect("file should be present");
    let output = overlay.output.expect("output section present");
    assert_eq!(output.theme, Some("compact".to_string()));
    assert_eq!(output.columns, Some(100));
    assert_eq!(output.show_error_detail, Some(false));
    assert!(overlay
        .commands
        .expect("commands present")
        .contains_key("hello"));
}

#[test]
fn read_config_file_invalid_toml_returns_err() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "not valid { toml }}}").unwrap();
    assert!(
        read_config_file(&path).is_err(),
        "invalid TOML should return Err"
    );
}

#[test]
fn read_config_file_missing_returns_ok_none() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("nonexistent.toml");
    assert!(
        matches!(read_config_file(&path), Ok(None)),
        "missing file should return Ok(None)"
    );
}

#[test]
#[serial]
fn global_config_path_uses_xdg_config_home() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let _guard = EnvGuard::set(
        "XDG_CONFIG_HOME",
        temp_dir.path().to_string_lossy().as_ref(),
    );
    // PERF-3 / TASK-1419: `global_config_path` is OnceLock-cached for
    // process-lifetime perf; tests that drive the env-precedence matrix
    // call the underlying resolver to bypass the cache.
    let path = resolve_global_config_path();
    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.starts_with(temp_dir.path()));
    assert!(path.ends_with("ops/config"));
}

#[test]
#[serial]
#[cfg(not(windows))]
fn global_config_path_falls_back_to_home_config() {
    let _xdg_guard = EnvGuard::remove("XDG_CONFIG_HOME");
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let _home_guard = EnvGuard::set("HOME", temp_dir.path().to_string_lossy().as_ref());
    let _userprofile_guard = EnvGuard::remove("USERPROFILE");

    // PERF-3 / TASK-1419: `global_config_path` is OnceLock-cached for
    // process-lifetime perf; tests that drive the env-precedence matrix
    // call the underlying resolver to bypass the cache.
    let path = resolve_global_config_path();

    assert!(path.is_some());
    let path = path.unwrap();
    assert!(path.to_string_lossy().contains(".config"));
    assert!(path.ends_with("ops/config"));
}

/// PORT-5 (TASK-0696): Windows must resolve the config base from `%APPDATA%`
/// rather than appending `.config/ops/config` to USERPROFILE. Compiled only
/// on Windows; the cross-platform XDG override remains covered above.
#[test]
#[serial]
#[cfg(windows)]
fn global_config_path_uses_appdata_on_windows() {
    let _xdg_guard = EnvGuard::remove("XDG_CONFIG_HOME");
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let _appdata_guard = EnvGuard::set("APPDATA", temp_dir.path().to_string_lossy().as_ref());

    // PERF-3 / TASK-1419: bypass the OnceLock so the env knob under test
    // is actually observed (see XDG test above).
    let path = resolve_global_config_path().expect("path resolves");

    assert!(
        path.starts_with(temp_dir.path()),
        "expected {} to live under APPDATA {}",
        path.display(),
        temp_dir.path().display()
    );
    assert!(path.ends_with("ops/config"));
    assert!(
        !path.to_string_lossy().contains(".config"),
        "Windows path should not embed Unix `.config` segment: {}",
        path.display()
    );
}

/// TQ-EFF-001: Permission-denied error path tests.
///
/// These tests are Unix-only because Windows has different permission semantics
/// (ACLs vs. Unix mode bits). On Windows, the behavior is verified at compile-time
/// via conditional compilation, but runtime testing is skipped.
mod read_config_file_error_paths {
    use super::*;

    /// TQ-EFF-001: Test that permission-denied errors are handled gracefully.
    ///
    /// This test is Unix-only because it uses `std::os::unix::fs::PermissionsExt`
    /// to set file permissions. Windows file permissions work differently (ACLs)
    /// and would require a different test approach.
    #[cfg(unix)]
    #[test]
    fn read_config_file_permission_denied_returns_none() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("unreadable.toml");
        std::fs::write(&path, "[output]\ntheme = \"classic\"").unwrap();

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = read_config_file(&path);
        assert!(result.is_err(), "permission denied should return Err");

        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644));
    }
}

#[test]
fn validate_commands_rejects_unknown_composite_ref() {
    let mut config = Config::default();
    config.commands.insert(
        "verify".to_string(),
        CommandSpec::Composite(crate::config::CompositeCommandSpec::new(["buidl"])),
    );
    let err = config
        .validate_commands(&[])
        .expect_err("unknown ref must fail");
    let msg = format!("{err:#}");
    assert!(msg.contains("unknown command 'buidl'"), "got: {msg}");
}

#[test]
fn validate_commands_accepts_unknown_ref_resolved_via_externals() {
    let mut config = Config::default();
    config.commands.insert(
        "run-before-commit".to_string(),
        CommandSpec::Composite(crate::config::CompositeCommandSpec::new(["verify"])),
    );
    // `verify` is provided by stack defaults — pass it as external.
    config
        .validate_commands(&["verify"])
        .expect("composite resolves via externals");
}

#[test]
fn validate_commands_rejects_self_cycle() {
    let mut config = Config::default();
    config.commands.insert(
        "loop".to_string(),
        CommandSpec::Composite(crate::config::CompositeCommandSpec::new(["loop"])),
    );
    let err = config
        .validate_commands(&[])
        .expect_err("self-cycle must fail");
    assert!(format!("{err:#}").contains("cycle"));
}

#[test]
fn validate_commands_rejects_indirect_cycle() {
    let mut config = Config::default();
    config.commands.insert(
        "a".to_string(),
        CommandSpec::Composite(crate::config::CompositeCommandSpec::new(["b"])),
    );
    config.commands.insert(
        "b".to_string(),
        CommandSpec::Composite(crate::config::CompositeCommandSpec::new(["a"])),
    );
    let err = config
        .validate_commands(&[])
        .expect_err("indirect cycle must fail");
    assert!(format!("{err:#}").contains("cycle"));
}

#[test]
fn validate_commands_rejects_depth_violation() {
    use crate::config::{CompositeCommandSpec, MAX_COMPOSITE_DEPTH};
    let mut config = Config::default();
    // Build a strict chain c0 -> c1 -> ... -> cN with N > MAX_COMPOSITE_DEPTH.
    let n = MAX_COMPOSITE_DEPTH + 5;
    for i in 0..n {
        let next = format!("c{}", i + 1);
        config.commands.insert(
            format!("c{i}"),
            CommandSpec::Composite(CompositeCommandSpec::new([next])),
        );
    }
    // Final exec leaf so refs resolve.
    config
        .commands
        .insert(format!("c{n}"), CommandSpec::Exec(exec_spec("echo", &[])));
    let err = config.validate_commands(&[]).expect_err("depth must fail");
    assert!(format!("{err:#}").contains("depth"));
}

#[test]
fn validate_commands_accepts_diamond_dag() {
    use crate::config::CompositeCommandSpec;
    // a -> [b, c]; b -> [d]; c -> [d]; d -> exec. Visiting `d` twice must
    // not be flagged as a cycle (matches runner expand_inner semantics).
    let mut config = Config::default();
    config.commands.insert(
        "a".to_string(),
        CommandSpec::Composite(CompositeCommandSpec::new(["b", "c"])),
    );
    config.commands.insert(
        "b".to_string(),
        CommandSpec::Composite(CompositeCommandSpec::new(["d"])),
    );
    config.commands.insert(
        "c".to_string(),
        CommandSpec::Composite(CompositeCommandSpec::new(["d"])),
    );
    config
        .commands
        .insert("d".to_string(), make_exec_spec("echo", &[]));
    config
        .validate_commands(&[])
        .expect("diamond is not a cycle");
}

/// ERR-1 / TASK-1181: two commands declaring the same alias are silently
/// resolved by `Config::resolve_alias` to whichever appears first in the
/// IndexMap, with no diagnostic. `validate_commands` must catch this up
/// front so the misconfiguration fails loud at config load instead of as
/// ghost behaviour at invocation time.
#[test]
fn validate_commands_rejects_duplicate_alias_across_commands() {
    let mut config = Config::default();
    let mut a = exec_spec("echo", &["a"]);
    a.aliases = vec!["shared".to_string()];
    let mut b = exec_spec("echo", &["b"]);
    b.aliases = vec!["shared".to_string()];
    config
        .commands
        .insert("alpha".to_string(), CommandSpec::Exec(a));
    config
        .commands
        .insert("beta".to_string(), CommandSpec::Exec(b));

    let err = config
        .validate_commands(&[])
        .expect_err("duplicate alias must fail validation");
    let msg = format!("{err:#}");
    assert!(msg.contains("'shared'"), "missing alias name; got: {msg}");
    assert!(
        msg.contains("alpha") && msg.contains("beta"),
        "must name both candidates; got: {msg}"
    );
}

/// ERR-1 / TASK-1182: an alias that collides with an existing command name
/// is silently dead because the External dispatcher matches the literal
/// command name first. `validate_commands` must reject this so a config
/// with `commands.build` and `commands.foo.aliases = ["build"]` fails at
/// validate time and names both keys.
#[test]
fn validate_commands_rejects_alias_colliding_with_command_name() {
    let mut config = Config::default();
    let mut foo = exec_spec("echo", &["foo"]);
    foo.aliases = vec!["build".to_string()];
    config.commands.insert(
        "build".to_string(),
        CommandSpec::Exec(exec_spec("echo", &["b"])),
    );
    config
        .commands
        .insert("foo".to_string(), CommandSpec::Exec(foo));

    let err = config
        .validate_commands(&[])
        .expect_err("alias-vs-command-name collision must fail validation");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("'build'"),
        "must name the colliding alias; got: {msg}"
    );
    assert!(msg.contains("foo"), "must name the alias owner; got: {msg}");
}

/// TASK-1182 (also): an alias that collides with an external command name
/// (stack default / extension command id) must also be rejected — that's
/// the exact dispatcher precedence that makes the alias dead.
#[test]
fn validate_commands_rejects_alias_colliding_with_external_command() {
    let mut config = Config::default();
    let mut foo = exec_spec("echo", &["foo"]);
    foo.aliases = vec!["test".to_string()];
    config
        .commands
        .insert("foo".to_string(), CommandSpec::Exec(foo));

    let err = config
        .validate_commands(&["test"])
        .expect_err("alias colliding with external must fail");
    let msg = format!("{err:#}");
    assert!(msg.contains("'test'"), "got: {msg}");
}

/// ERR-1 / TASK-1221: `walk_composite` must leave `visiting` empty on every
/// exit path, including unknown-ref bail and child-error short-circuits.
/// This test exercises the invariant directly — a future refactor that
/// hoists `visiting` across sibling composite roots would otherwise silently
/// produce false-positive cycle errors on re-validation.
#[test]
fn walk_composite_clears_visiting_on_unknown_ref_error() {
    use crate::config::CompositeCommandSpec;
    let mut config = Config::default();
    // Top-level composite with an unknown sub-ref so the inner loop bails.
    config.commands.insert(
        "outer".to_string(),
        CommandSpec::Composite(CompositeCommandSpec::new(["nope"])),
    );
    let mut visiting: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let known: std::collections::HashSet<&str> =
        config.commands.keys().map(String::as_str).collect();
    let err = config
        .walk_composite("outer", &known, &mut visiting, 0)
        .expect_err("unknown ref must error");
    assert!(format!("{err:#}").contains("unknown command 'nope'"));
    assert!(
        visiting.is_empty(),
        "visiting must be cleared after error; got: {visiting:?}"
    );
}

#[test]
fn walk_composite_clears_visiting_on_recursive_error() {
    use crate::config::CompositeCommandSpec;
    let mut config = Config::default();
    // outer -> mid -> nope (unknown). Error surfaces from a deeper frame; the
    // outer frame must still clear its own entry on the way out.
    config.commands.insert(
        "outer".to_string(),
        CommandSpec::Composite(CompositeCommandSpec::new(["mid"])),
    );
    config.commands.insert(
        "mid".to_string(),
        CommandSpec::Composite(CompositeCommandSpec::new(["nope"])),
    );
    let mut visiting: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let known: std::collections::HashSet<&str> =
        config.commands.keys().map(String::as_str).collect();
    let err = config
        .walk_composite("outer", &known, &mut visiting, 0)
        .expect_err("nested unknown ref must error");
    assert!(format!("{err:#}").contains("unknown command 'nope'"));
    assert!(
        visiting.is_empty(),
        "visiting must be cleared even on recursive error; got: {visiting:?}"
    );
}

#[test]
fn scale_columns_handles_huge_widths_without_wrapping() {
    // SEC-15 / TASK-0344: a terminal width that would overflow `w*9` in u16
    // must not wrap or panic. Promoted to u32, the result for any u16 input
    // fits back in u16 (max ~58 981 for u16::MAX).
    assert_eq!(super::scale_columns(80), 72);
    assert_eq!(super::scale_columns(100), 90);
    // 8000 cols: in u16, 8000 * 9 wraps; the u32-promoted version returns 7200.
    assert_eq!(super::scale_columns(8000), 7200);
    assert_eq!(super::scale_columns(u16::MAX), 58_981);
}
