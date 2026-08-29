use super::*;
use crate::test_utils::{exec_spec, EnvGuard};
use serial_test::serial;
use std::time::Duration;

#[test]
fn exec_spec_timeout_some() {
    let mut e = exec_spec("cargo", &["build"]);
    e.timeout_secs = Some(300);
    assert_eq!(e.timeout(), Some(Duration::from_mins(5)));
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

/// `display_program` overrides only the rendered program name; args (and
/// their SEC-21 quoting) are unaffected. Builtins rely on this to render
/// `ops sec` while spawning the absolute `current_exe()` path.
#[test]
fn exec_spec_display_cmd_prefers_display_program() {
    let mut with_args = exec_spec("/home/me/.cargo/bin/ops", &["sec", "--skip", "vuln"]);
    with_args.display_program = Some("ops".to_string());
    assert_eq!(with_args.display_cmd(), "ops sec --skip vuln");

    let mut no_args = exec_spec("/home/me/.cargo/bin/ops", &[]);
    no_args.display_program = Some("ops".to_string());
    assert_eq!(no_args.display_cmd(), "ops");

    // An override containing shell metacharacters is quoted like any program.
    let mut unsafe_display = exec_spec("/bin/echo", &["hi"]);
    unsafe_display.display_program = Some("my ops".to_string());
    assert_eq!(unsafe_display.display_cmd(), "'my ops' hi");
}

/// SEC-21: `display_program` is internal (builtins only). A `.ops.toml`
/// supplying it must fail to load — a config-settable display name that
/// diverges from the real program could disguise what actually runs.
#[test]
fn exec_spec_display_program_is_not_config_settable() {
    let toml = r#"
program = "cargo"
args = ["build"]
display_program = "harmless-looking"
"#;
    let err = toml::from_str::<ExecCommandSpec>(toml)
        .expect_err("display_program must be rejected in config");
    assert!(
        err.to_string().contains("unknown field"),
        "expected unknown-field error, got: {err}"
    );
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

/// TASK-1431: `cwd` containing `..` must be rejected at load time so a
/// hostile workspace config can't silently escape the workspace at exec.
#[test]
fn exec_spec_validate_rejects_cwd_with_parent_dir() {
    let mut e = exec_spec("echo", &["hi"]);
    e.cwd = Some(std::path::PathBuf::from("../../etc"));
    let err = e.validate("bad").unwrap_err().to_string();
    assert!(err.contains("cwd"), "expected cwd error, got: {err}");
    assert!(err.contains(".."), "expected '..' mention, got: {err}");
}

/// TASK-1445: NUL or other C0 control characters in program/args/cwd
/// must fail at validate-time with a named error rather than at spawn.
#[test]
fn exec_spec_validate_rejects_nul_in_program() {
    let e = exec_spec("ec\u{0}ho", &["hi"]);
    let err = e.validate("bad").unwrap_err().to_string();
    assert!(err.contains("program"), "expected program error: {err}");
    assert!(
        err.contains("control character"),
        "expected control char mention: {err}"
    );
}

#[test]
fn exec_spec_validate_rejects_newline_in_args() {
    let e = exec_spec("echo", &["bad\narg"]);
    let err = e.validate("bad").unwrap_err().to_string();
    assert!(err.contains("args[0]"), "expected args[0] error: {err}");
}

#[test]
fn exec_spec_validate_rejects_control_in_cwd() {
    let mut e = exec_spec("echo", &[]);
    e.cwd = Some(std::path::PathBuf::from("dir\u{0}sub"));
    let err = e.validate("bad").unwrap_err().to_string();
    assert!(err.contains("cwd"), "expected cwd error: {err}");
}

/// TASK-1445: tab is explicitly allowed (common in legitimate args).
#[test]
fn exec_spec_validate_allows_tab() {
    let e = exec_spec("echo", &["a\tb"]);
    assert!(e.validate("ok").is_ok());
}

/// SEC-11 / TASK-1826: a NUL in an `env` **value** reaches `Command::env`
/// unscreened before the fix and surfaces as std's anonymous "nul byte found
/// in provided data". Validation must name the command and the variable.
#[test]
fn exec_spec_validate_rejects_nul_in_env_value() {
    let mut e = exec_spec("echo", &["hi"]);
    e.env.insert("TOKEN".to_string(), "se\u{0}cret".to_string());
    let err = e.validate("deploy").unwrap_err().to_string();
    assert!(err.contains("deploy"), "expected command name, got: {err}");
    assert!(err.contains("env[TOKEN]"), "expected the key, got: {err}");
    assert!(
        err.contains("control character"),
        "expected control char mention: {err}"
    );
}

/// SEC-11 / TASK-1826: a control character in an env **key** is rejected too —
/// the key is half of what `Command::env` receives.
#[test]
fn exec_spec_validate_rejects_control_char_in_env_key() {
    let mut e = exec_spec("echo", &["hi"]);
    e.env.insert("BA\nD".to_string(), "x".to_string());
    let err = e.validate("deploy").unwrap_err().to_string();
    assert!(err.contains("deploy"), "expected command name, got: {err}");
    assert!(err.contains("env key"), "expected env-key mention: {err}");
}

/// SEC-11 / TASK-1826: an `=` in an env key produces an entry the child's
/// `getenv` can never retrieve, so it fails at load naming the key.
#[test]
fn exec_spec_validate_rejects_equals_in_env_key() {
    let mut e = exec_spec("echo", &["hi"]);
    e.env.insert("A=B".to_string(), "x".to_string());
    let err = e.validate("deploy").unwrap_err().to_string();
    assert!(err.contains("deploy"), "expected command name, got: {err}");
    assert!(err.contains("A=B"), "expected the key, got: {err}");
    assert!(err.contains('='), "expected the '=' rule, got: {err}");
}

/// SEC-11 / TASK-1826: an ordinary env map still validates — the new screen
/// must not reject legitimate configs (tab stays allowed, as in `args`).
#[test]
fn exec_spec_validate_accepts_plain_env() {
    let mut e = exec_spec("echo", &["hi"]);
    e.env.insert("RUST_LOG".to_string(), "debug".to_string());
    e.env.insert("TABBED".to_string(), "a\tb".to_string());
    e.validate("ok").expect("plain env must validate");
}

/// TASK-1430: typo of the `program` field reports the Exec error, not
/// the misleading Composite "missing field `commands`".
#[test]
fn command_spec_typo_in_exec_reports_exec_error() {
    let toml_str = r#"
[commands.build]
progam = "cargo"
"#;
    let err = toml::from_str::<crate::config::Config>(toml_str)
        .expect_err("typo'd program field must not parse");
    let msg = err.to_string();
    assert!(
        msg.contains("progam") || msg.contains("unknown field"),
        "expected Exec-side error mentioning typo'd field, got: {msg}"
    );
    assert!(
        !msg.contains("missing field `commands`"),
        "must not report misleading Composite error, got: {msg}"
    );
}

/// TASK-1430: typo on Composite side reports the Composite error.
#[test]
fn command_spec_typo_in_composite_reports_composite_error() {
    let toml_str = r#"
[commands.ci]
commands = ["build", "test"]
parralel = true
"#;
    let err = toml::from_str::<crate::config::Config>(toml_str)
        .expect_err("typo'd parallel field must not parse");
    let msg = err.to_string();
    assert!(
        msg.contains("parralel") || msg.contains("unknown field"),
        "expected Composite-side error, got: {msg}"
    );
}

/// TASK-1402: `CommandId` is parseable via the standard `str::parse` path.
#[test]
fn command_id_from_str() {
    let id: crate::config::CommandId = "build".parse().expect("infallible");
    assert_eq!(id.as_str(), "build");
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

    /// TQ-EFF-001 / TEST-2 (TASK-1835): an unreadable config file must fail
    /// **loud** — `Err`, not the `Ok(None)` that means "file absent". The two
    /// outcomes are the fail-closed / fail-open fork of the config reader, and
    /// the sibling `read_config_file_missing_returns_ok_none` pins the other
    /// side of it. The name says `returns_err` because that is what the body
    /// asserts; the previous name documented the opposite contract.
    ///
    /// This test is Unix-only because it uses `std::os::unix::fs::PermissionsExt`
    /// to set file permissions. Windows file permissions work differently (ACLs)
    /// and would require a different test approach.
    ///
    /// TEST-18 (TASK-1835): `chmod 0o000` is bypassed by `CAP_DAC_OVERRIDE`, so
    /// under a privileged sandbox (CI as root, fakeroot) the read succeeds and
    /// there is nothing to assert. Detect that case the way
    /// `edit.rs::sync_parent_dir_warns_when_parent_open_fails` does — probe the
    /// open first, and only assert the refusal when the OS actually denied it.
    #[cfg(unix)]
    #[test]
    fn read_config_file_permission_denied_returns_err() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("unreadable.toml");
        std::fs::write(&path, "[output]\ntheme = \"classic\"").unwrap();

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        // Independent probe: does this process actually get EACCES here?
        let dac_enforced = std::fs::File::open(&path)
            .err()
            .is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied);
        let result = read_config_file(&path);

        // Restore permissions before any assertion so a failure cannot leave a
        // 0o000 file behind for the `TempDir` teardown.
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).ok();

        if dac_enforced {
            let err = result.expect_err("permission denied must return Err, never Ok(None)");
            assert!(
                format!("{err:#}").contains("unreadable.toml"),
                "error must name the offending file, got: {err:#}"
            );
        } else {
            // Privileged sandbox: DAC was bypassed, so the read is expected to
            // succeed. Pin that it is not silently swallowed as "absent".
            let overlay = result.expect("privileged read must succeed");
            assert!(
                overlay.is_some(),
                "a readable file must never map to Ok(None)"
            );
        }
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
/// `IndexMap`, with no diagnostic. `validate_commands` must catch this up
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
    let mut state = crate::config::root::CompositeWalk::default();
    let known: std::collections::HashSet<&str> =
        config.commands.keys().map(String::as_str).collect();
    let err = config
        .walk_composite("outer", &known, &mut state, 0)
        .expect_err("unknown ref must error");
    assert!(format!("{err:#}").contains("unknown command 'nope'"));
    assert!(
        state.path_is_empty(),
        "visiting must be cleared after error; got: {state:?}"
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
    let mut state = crate::config::root::CompositeWalk::default();
    let known: std::collections::HashSet<&str> =
        config.commands.keys().map(String::as_str).collect();
    let err = config
        .walk_composite("outer", &known, &mut state, 0)
        .expect_err("nested unknown ref must error");
    assert!(format!("{err:#}").contains("unknown command 'nope'"));
    assert!(
        state.path_is_empty(),
        "visiting must be cleared even on recursive error; got: {state:?}"
    );
}

#[test]
fn scale_columns_handles_huge_widths_without_wrapping() {
    // SEC-15 / TASK-0344: a terminal width that would overflow `w*9` in u16
    // must not wrap or panic. Promoted to u32, the result for any u16 input
    // fits back in u16 (max ~58 981 for u16::MAX).
    assert_eq!(scale_columns(80), 72);
    assert_eq!(scale_columns(100), 90);
    // 8000 cols: in u16, 8000 * 9 wraps; the u32-promoted version returns 7200.
    assert_eq!(scale_columns(8000), 7200);
    assert_eq!(scale_columns(u16::MAX), 58_981);
}

/// SEC-31 / TASK-1818: the two alias-hygiene rules must run in the **shipped
/// binary**. They lived only in `validate_commands`, which has no production
/// caller, so a `.ops.toml` declaring the same alias twice loaded cleanly and
/// then dispatched to whichever command happened to sit earlier in the
/// `IndexMap` — silently running the wrong command. This test goes through the
/// real load entry point, not a direct `validate_commands` call.
#[test]
#[serial]
fn load_config_rejects_duplicate_alias_across_commands() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Isolates XDG and clears the resolver cache both on entry and on drop,
    // so the cached tempdir path cannot outlive this test.
    let _xdg = crate::test_utils::isolate_global_config(dir.path());
    let _env = EnvGuard::remove("OPS__OUTPUT__THEME");
    std::fs::write(
        dir.path().join(".ops.toml"),
        r#"
[commands.build]
program = "cargo"
aliases = ["b"]

[commands.bench]
program = "cargo"
aliases = ["b"]
"#,
    )
    .unwrap();

    let err = crate::config::load_config_at(dir.path())
        .expect_err("a duplicate alias must fail the real load path");

    let msg = format!("{err:#}");
    assert!(msg.contains("'b'"), "error must name the alias, got: {msg}");
    assert!(
        msg.contains("build") && msg.contains("bench"),
        "error must name both owners, got: {msg}"
    );
}

/// SEC-31 / TASK-1818: the symmetric rule — an alias shadowing an existing
/// command name is silently dead at dispatch, and must fail the real load
/// path rather than at invocation time.
#[test]
#[serial]
fn load_config_rejects_alias_shadowing_a_command_name() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Isolates XDG and clears the resolver cache both on entry and on drop,
    // so the cached tempdir path cannot outlive this test.
    let _xdg = crate::test_utils::isolate_global_config(dir.path());
    let _env = EnvGuard::remove("OPS__OUTPUT__THEME");
    std::fs::write(
        dir.path().join(".ops.toml"),
        r#"
[commands.build]
program = "cargo"

[commands.bench]
program = "cargo"
aliases = ["build"]
"#,
    )
    .unwrap();

    let err = crate::config::load_config_at(dir.path())
        .expect_err("an alias shadowing a command name must fail the real load path");

    let msg = format!("{err:#}");
    assert!(
        msg.contains("collides with an existing command name"),
        "error must explain the collision, got: {msg}"
    );
    assert!(msg.contains("build"), "error must name the alias: {msg}");
}

/// SEC-33 / TASK-1832: `walk_composite` kept no memory of subtrees it had
/// already validated, so every extra incoming edge re-walked the whole
/// subtree. A 31-command `.ops.toml` where each composite lists the next one
/// twice therefore cost 2^30 visits and hung `ops <anything>` — an
/// unauthenticated local `DoS` from repo-supplied config, well inside every size
/// cap. With the `validated` memo the walk is O(V+E) and returns instantly.
///
/// TEST-15: the budget is deliberately enormous (five seconds against a walk
/// that now takes microseconds) so the test cannot flake on a loaded CI box
/// while still failing decisively if the memo is dropped — 2^30 visits do not
/// complete in five seconds on any machine.
#[test]
fn validate_commands_doubling_chain_is_not_exponential() {
    use crate::config::CompositeCommandSpec;
    let mut config = Config::default();
    let depth = 30;
    for i in 0..depth {
        let next = format!("c{}", i + 1);
        config.commands.insert(
            format!("c{i}"),
            // Each composite lists its successor *twice*: without memoisation
            // this doubles the visit count at every level.
            CommandSpec::Composite(CompositeCommandSpec::new([next.clone(), next])),
        );
    }
    config.commands.insert(
        format!("c{depth}"),
        CommandSpec::Exec(exec_spec("true", &[])),
    );

    let started = std::time::Instant::now();
    config
        .validate_commands(&[])
        .expect("a doubling chain is a DAG, not a cycle");
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "validation must be linear in the command count, took {elapsed:?}"
    );
}

/// SEC-33 / TASK-1832: memoisation must not weaken the depth limit. A node
/// validated at a shallow depth can still blow `MAX_COMPOSITE_DEPTH` when
/// reached again from a deeper root, so the memo stores each subtree's height
/// and re-checks `depth + height` on every hit. A bare "seen" flag would fail
/// open here.
#[test]
fn validate_commands_memo_still_enforces_depth_limit() {
    use crate::config::CompositeCommandSpec;
    let mut config = Config::default();
    // A straight chain c0 -> c1 -> ... -> cN with N > MAX_COMPOSITE_DEPTH.
    // `validate_commands` walks every composite as a root, so the deepest
    // suffixes memoise first and the full-length root then hits the memo.
    let n = MAX_COMPOSITE_DEPTH + 5;
    for i in 0..n {
        config.commands.insert(
            format!("c{i}"),
            CommandSpec::Composite(CompositeCommandSpec::new([format!("c{}", i + 1)])),
        );
    }
    config
        .commands
        .insert(format!("c{n}"), CommandSpec::Exec(exec_spec("true", &[])));

    let err = config
        .validate_commands(&[])
        .expect_err("an over-deep chain must still be rejected");
    assert!(
        format!("{err:#}").contains("depth"),
        "expected a depth-limit error, got: {err:#}"
    );
}

/// SEC-33 / TASK-1832, direct form: the sibling test above drives
/// `validate_commands`, which only reaches the memo *incidentally* — a change
/// to root ordering could make it stop exercising the memo-hit branch at all
/// while still passing. This one calls [`Config::walk_composite`] itself:
/// walk `mid` once at depth 0 so its height memoises, then re-enter the very
/// same node at [`MAX_COMPOSITE_DEPTH`] with the same [`CompositeWalk`]. Only
/// the `depth + height` re-check on the memo hit can reject that; a bare
/// "seen" flag returns `Ok` and fails open.
#[test]
fn walk_composite_memo_hit_rechecks_depth() {
    use crate::config::root::CompositeWalk;
    use crate::config::CompositeCommandSpec;
    let mut config = Config::default();
    // mid -> leaf, so `mid` memoises with height 1 rather than 0 and the
    // re-check has something to add to the entry depth.
    config.commands.insert(
        "mid".to_string(),
        CommandSpec::Composite(CompositeCommandSpec::new(["leaf"])),
    );
    config.commands.insert(
        "leaf".to_string(),
        CommandSpec::Composite(CompositeCommandSpec::new(Vec::<String>::new())),
    );
    let known: std::collections::HashSet<&str> = ["mid", "leaf"].into_iter().collect();

    let mut state = CompositeWalk::default();
    let height = config
        .walk_composite("mid", &known, &mut state, 0)
        .expect("a two-node chain validates at depth 0");
    assert_eq!(height, 1, "mid -> leaf is one edge tall");
    assert!(state.path_is_empty(), "the DFS path must unwind fully");

    let err = config
        .walk_composite("mid", &known, &mut state, MAX_COMPOSITE_DEPTH)
        .expect_err("a memoised node re-entered at the limit still overflows it");
    assert!(
        format!("{err:#}").contains("depth"),
        "expected a depth-limit error, got: {err:#}"
    );
}

/// SEC-33 / TASK-1849: `left_pad` sizes `" ".repeat(n)` in three renderers,
/// and nothing validated `[themes]`, so a ~400-byte `.ops.toml` could panic
/// the process with a capacity overflow. It must now be a clean `Err` from the
/// real load path.
#[test]
#[serial]
fn load_config_rejects_usize_max_left_pad() {
    let err = load_config_with_left_pad(&usize::MAX.to_string());
    assert!(
        err.contains("left_pad"),
        "error must name the field, got: {err}"
    );
}

/// SEC-33 / TASK-1849: the allocation-failure shape (`memory allocation of
/// 50000000000 bytes failed`, an abort) is rejected the same way.
#[test]
#[serial]
fn load_config_rejects_huge_left_pad() {
    let err = load_config_with_left_pad("50000000000");
    assert!(
        err.contains("left_pad"),
        "error must name the field, got: {err}"
    );
}

/// SEC-33 / TASK-1849: a realistic margin still loads — the bound must not
/// reject legitimate themes.
#[test]
#[serial]
fn load_config_accepts_ordinary_left_pad() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Isolates XDG and clears the resolver cache both on entry and on drop,
    // so the cached tempdir path cannot outlive this test.
    let _xdg = crate::test_utils::isolate_global_config(dir.path());
    let _env = EnvGuard::remove("OPS__OUTPUT__THEME");
    std::fs::write(dir.path().join(".ops.toml"), theme_toml("4")).unwrap();

    let config = crate::config::load_config_at(dir.path()).expect("an ordinary left_pad must load");
    assert_eq!(config.themes["wide"].left_pad, 4);
}

/// SEC-33 / TASK-1849: `ThemeConfig::validate` is the screen for a
/// programmatically-built theme, which never passes through serde. Its message
/// names the theme so an operator can find it.
#[test]
fn theme_validate_rejects_out_of_range_left_pad_and_names_the_theme() {
    use crate::config::theme_types::MAX_LEFT_PAD;
    let mut theme = crate::config::theme_types::ThemeConfig::classic();
    theme.left_pad = MAX_LEFT_PAD + 1;

    let err = theme
        .validate("gigantic")
        .expect_err("an out-of-range left_pad must be rejected")
        .to_string();

    assert!(err.contains("gigantic"), "must name the theme, got: {err}");
    assert!(err.contains("left_pad"), "must name the field, got: {err}");
}

/// A `.ops.toml` defining and selecting a theme with the given `left_pad`.
fn theme_toml(left_pad: &str) -> String {
    format!(
        r#"
[output]
theme = "wide"

[themes.wide]
icon_pending = "o"
icon_running = ""
icon_succeeded = "+"
icon_failed = "x"
icon_skipped = "-"
separator_char = "."
step_indent = "  "
running_template = "  {{spinner}}{{msg}} {{elapsed}}"
tick_chars = "|/-\\ "
running_template_overhead = 7
summary_prefix = ""
summary_separator = ""
left_pad = {left_pad}
"#
    )
}

/// Load a `.ops.toml` carrying `left_pad` and return the rendered error chain.
fn load_config_with_left_pad(left_pad: &str) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    // Isolates XDG and clears the resolver cache both on entry and on drop,
    // so the cached tempdir path cannot outlive this test.
    let _xdg = crate::test_utils::isolate_global_config(dir.path());
    let _env = EnvGuard::remove("OPS__OUTPUT__THEME");
    std::fs::write(dir.path().join(".ops.toml"), theme_toml(left_pad)).unwrap();

    let err = crate::config::load_config_at(dir.path())
        .expect_err("an out-of-range left_pad must be a clean Err, not a panic or an abort");
    format!("{err:#}")
}
