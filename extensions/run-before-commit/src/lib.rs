//! Run-before-commit hook extension: install and manage git pre-commit hooks.

use std::time::Duration;

use ops_extension::ExtensionType;

/// Extension identifier used to register this crate in the engine's
/// extension registry.
///
/// Doubles as the on-disk spelling the installed hook script execs
/// (`exec ops run-before-commit --changed-only`) and the `.ops.toml`
/// `[commands.<name>]` key `ensure_config_command` writes, so all three
/// agree on one string.
pub const NAME: &str = "run-before-commit";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "Setup git pre-commit hook to run an ops command of your choice";
/// CLI-facing short name — the subcommand the user types.
///
/// Equal to `NAME` here (no reason to spell them differently), unlike
/// crates where a stack suffix makes the full name too long to type.
pub const SHORTNAME: &str = "run-before-commit";

/// Command extension wiring the pre-commit install and dispatch surface.
///
/// Registers no data providers and no commands of its own; the CLI reaches
/// this crate's install entry points through `crates/cli/src/pre_hook_cmd.rs`.
pub struct RunBeforeCommitExtension;

ops_extension::impl_extension! {
    RunBeforeCommitExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    data_provider_name: None,
    register_data_providers: |_self, _registry| {},
    factory: RUN_BEFORE_COMMIT_FACTORY = |_, _| {
        Some((NAME, Box::new(RunBeforeCommitExtension)))
    },
}

/// The shell script installed as `.git/hooks/pre-commit`.
///
/// Four properties are load-bearing and covered by tests below:
///
/// 1. **`#!/bin/sh`, not bash** — the body uses nothing bash provides, and a
///    bash dependency breaks the hook on busybox/Alpine images, minimal
///    container builds and BSD/Nix setups without bash in scope, where `env`
///    exits 127 and git blocks every commit with a message naming bash rather
///    than ops. The legacy hooks this crate recognises and replaces are all
///    `#!/bin/sh` already.
/// 2. **`ops` is probed before it is exec'd** — git hooks inherit the
///    environment of whatever invoked git, and GUI clients (IDE VCS panes,
///    GitHub Desktop, `SourceTree`, Fork) launch from the desktop session, so
///    `~/.cargo/bin` and `~/.local/bin` are routinely absent. A bare
///    `ops: command not found` is a 127 in a dialog box that names neither the
///    tool nor the fix, and the user's only escape is deleting the hook by
///    hand. The guard names ops, the hook path, and the bypass env var.
/// 3. **The bypass is honoured before the probe** — the probe's own
///    diagnostic advertises `SKIP_OPS_RUN_BEFORE_COMMIT`, so it has to work
///    in exactly the situation the diagnostic describes. The bypass-then-
///    probe prologue is shared with the pre-push hook through
///    [`ops_hook_common::hook_script!`], not copy-pasted.
/// 4. **`--changed-only`** — that flag is what arms the [`has_staged_files`]
///    preflight (`crates/cli/src/subcommands.rs`), so an empty index skips the
///    configured command chain instead of paying for a full check suite.
///    Without it the bounded-wait probe this crate parameterises below is
///    unreachable from the installed hook, and docs/commands.md's "skips when
///    nothing is staged" would be a promise the hook does not keep.
const HOOK_SCRIPT: &str = ops_hook_common::hook_script! {
    name: "run-before-commit",
    hook_filename: "pre-commit",
    skip_env_var: "SKIP_OPS_RUN_BEFORE_COMMIT",
    tail: "exec ops run-before-commit --changed-only\n",
};

/// Environment variable that skips the run-before-commit check.
///
/// Recognized values are `1`, `true`, `yes` and `on`, matched
/// case-insensitively; anything else — including the empty string, `0` and
/// `false` — means "do not skip". [`ops_hook_common::should_skip`] is the
/// source of truth for that list; keep this doc in step with it.
pub const SKIP_ENV_VAR: &str = "SKIP_OPS_RUN_BEFORE_COMMIT";

ops_hook_common::impl_hook_wrappers! {
    name: NAME,
    hook_filename: "pre-commit",
    hook_script: HOOK_SCRIPT,
    skip_env_var: SKIP_ENV_VAR,
    legacy_markers: &[
        "ops run-before-commit",
        "ops before-commit",
        "ops pre-commit",
    ],
    command_help: "Run run-before-commit checks before committing",
}

/// Pre-commit hooks run on the developer's critical path, so the git probe
/// is bounded-wait. The bounded-wait probe lives in
/// `ops_hook_common::git_state`; this crate parameterises it with
/// hook-specific constants.
const DEFAULT_GIT_TIMEOUT: Duration = Duration::from_secs(5);
const TIMEOUT_ENV_VAR: &str = "OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS";

/// Upper bound on `OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS`. 300 s is
/// generous for even the slowest FUSE-backed worktree while still bounding
/// the hook.
const MAX_GIT_TIMEOUT_SECS: u64 = 300;

/// Returns `true` if the git index holds any staged change.
///
/// Every staged change kind counts — additions, modifications, renames,
/// **deletions**, type changes and unmerged paths — so a delete-only or
/// conflicted index never reads as "nothing staged" and never skips the
/// gate. See
/// [`ops_hook_common::git_state::has_staged_files_with_timeout`].
///
/// # Errors
///
/// If the current directory cannot be read, or the `git` probe fails or
/// times out.
pub fn has_staged_files() -> anyhow::Result<bool> {
    use anyhow::Context;
    let cwd = std::env::current_dir().context("failed to read current directory")?;
    let timeout = git_timeout_from_env().unwrap_or(DEFAULT_GIT_TIMEOUT);
    ops_hook_common::git_state::has_staged_files_with_timeout("git", &cwd, timeout)
        .map_err(anyhow::Error::from)
}

fn git_timeout_from_env() -> Option<Duration> {
    ops_hook_common::git_state::git_timeout_from_env(TIMEOUT_ENV_VAR, MAX_GIT_TIMEOUT_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::test_utils::git_fixture;
    use ops_hook_common::git_state::HasStagedFilesError;
    use ops_hook_common::test_helpers::{CwdGuard, EnvGuard, hermetic_git_env};
    use std::path::Path;

    // `EnvGuard`/`CwdGuard` mutate process-global state
    // (environ, cwd), and `std::process::Command` snapshots both while
    // building a child. `serial_test::serial` serializes its members against
    // *each other only*, so a test that spawns a subprocess without the
    // attribute can race a serial env/cwd mutator on another harness thread —
    // a setenv/getenv data race, not mere flakiness. Every test in this
    // module that spawns a child process (directly or through
    // `has_staged_files` or `git_fixture`) therefore MUST carry
    // `#[serial_test::serial]`, same key as the mutating tests. A new
    // spawning test that omits it silently opts out of this guarantee.

    // -- HOOK_SCRIPT --

    #[test]
    fn hook_script_contains_ops_run_before_commit() {
        assert!(HOOK_SCRIPT.contains("ops run-before-commit"));
    }

    /// The script must not depend on bash being installed, and no future
    /// edit may reintroduce the dependency.
    #[test]
    fn hook_script_uses_posix_sh_shebang() {
        assert!(
            HOOK_SCRIPT.starts_with("#!/bin/sh\n"),
            "HOOK_SCRIPT must not depend on bash, got: {HOOK_SCRIPT}"
        );
        assert!(!HOOK_SCRIPT.contains("bash"));
    }

    /// `ops` is resolved through an explicit probe that reports what is
    /// missing, not by exec'ing it and hoping.
    #[test]
    fn hook_script_guards_missing_ops_binary() {
        assert!(HOOK_SCRIPT.contains("command -v ops"));
        assert!(HOOK_SCRIPT.contains(SKIP_ENV_VAR));
        assert!(HOOK_SCRIPT.contains("exit 1"));
    }

    /// The shared prologue is generated from string literals, so the
    /// `SKIP_ENV_VAR` const and the literal spelled into the script must be
    /// pinned together — a macro cannot reference the const and stay
    /// `const`-evaluable. The expected script text is derived from the
    /// const, so drift on either side fails here.
    #[test]
    fn skip_env_var_const_matches_the_literal_spelled_into_the_script() {
        let bypass_check = format!("case \"${{{SKIP_ENV_VAR}:-}}\"");
        assert!(
            HOOK_SCRIPT.contains(&bypass_check),
            "the script must test {SKIP_ENV_VAR}, got: {HOOK_SCRIPT}"
        );
    }

    /// The installed hook arms the preflight, so docs/commands.md's "skips when
    /// nothing is staged" describes the shipped hook.
    #[test]
    fn hook_script_passes_changed_only() {
        assert!(
            HOOK_SCRIPT.contains("exec ops run-before-commit --changed-only"),
            "the installed hook must arm the has_staged_files preflight, got: {HOOK_SCRIPT}"
        );
    }

    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn hook_script_is_valid_posix_sh() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("pre-commit");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();
        let status = ops_core::subprocess::status_locked(
            std::process::Command::new("/bin/sh").arg("-n").arg(&script),
        )
        .unwrap();
        assert!(status.success(), "HOOK_SCRIPT must parse under `sh -n`");
    }

    /// With `ops` off PATH the hook must name ops and the reinstall
    /// command on stderr rather than surfacing a bare 127.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn hook_script_reports_a_missing_ops_binary_by_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("pre-commit");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();

        // PATH points only at this test's own empty
        // tempdir, so the `command -v ops` probe fails unconditionally — an
        // empty directory cannot contain `ops` on any machine, unlike
        // /usr/bin:/bin, which a distro package or CI image may well occupy.
        // The script's prologue is shell builtins only, so nothing else on
        // PATH is consulted before the probe.
        let out = ops_core::subprocess::output_locked(
            std::process::Command::new("/bin/sh")
                .arg(&script)
                .env("PATH", dir.path())
                // `Command` inherits the parent environment, so a bypass left
                // in the test runner's env (`SKIP_OPS_RUN_BEFORE_COMMIT=1
                // cargo test`) would exit the hook before the missing-ops
                // probe this test exists to exercise.
                .env_remove(SKIP_ENV_VAR),
        )
        .unwrap();

        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "stderr was: {stderr}");
        assert!(stderr.contains("ops"), "must name ops, got: {stderr}");
        assert!(stderr.contains(".git/hooks/pre-commit"), "must name the hook path, got: {stderr}");
        assert!(
            stderr.contains("ops run-before-commit install"),
            "must name the reinstall command, got: {stderr}"
        );
        assert!(stderr.contains(SKIP_ENV_VAR), "must name the bypass, got: {stderr}");
    }

    /// The missing-ops diagnostic names [`SKIP_ENV_VAR`] as the escape hatch,
    /// so the escape hatch must fire before the probe that prints it —
    /// otherwise the only advice a stuck user gets is advice that does not
    /// work. Driven with `ops` off PATH, which is the situation in question.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn hook_script_honours_the_bypass_when_ops_is_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("pre-commit");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();

        // PATH contains only the empty tempdir holding
        // the script, so `ops` is unfindable on every machine and the
        // fall-through can never exec a real ops binary.
        for value in ["1", "true", "TRUE", "Yes", "on"] {
            let out = ops_core::subprocess::output_locked(
                std::process::Command::new("/bin/sh")
                    .arg(&script)
                    .env("PATH", dir.path())
                    .env(SKIP_ENV_VAR, value),
            )
            .unwrap();
            assert_eq!(
                out.status.code(),
                Some(0),
                "{value:?} must skip cleanly, stderr was: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }

        // A value `should_skip` rejects must still reach the probe and fail.
        let out = ops_core::subprocess::output_locked(
            std::process::Command::new("/bin/sh")
                .arg(&script)
                .env("PATH", dir.path())
                .env(SKIP_ENV_VAR, "maybe"),
        )
        .unwrap();
        assert_eq!(out.status.code(), Some(1));
    }

    // -- should_skip --

    #[test]
    #[serial_test::serial]
    fn should_skip_returns_false_by_default() {
        let _guard = EnvGuard::remove(SKIP_ENV_VAR);
        assert!(!should_skip());
    }

    // -- install_hook: wrapper-specific legacy markers --

    /// Every marker on the legacy upgrade contract — the names earlier ops
    /// releases installed, including the one this crate itself writes — is
    /// upgraded in place rather than left alone as an unrecognised hook.
    /// Iterating [`HOOK_CONFIG.legacy_markers`] keeps a newly added marker
    /// covered without copying the test body.
    #[test]
    fn install_hook_updates_every_legacy_marker() {
        for marker in HOOK_CONFIG.legacy_markers {
            let dir = tempfile::tempdir().expect("tempdir");
            let git_dir = dir.path().join(".git");
            std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
            std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
            std::fs::write(git_dir.join("hooks/pre-commit"), format!("#!/bin/sh\nexec {marker}\n"))
                .unwrap();

            let mut buf = Vec::new();
            let path = install_hook(&git_dir, &mut buf).expect("install_hook");

            // The returned path must be the pre-commit hook
            // git actually runs — a wrong `hook_filename` in `HOOK_CONFIG` would
            // write a different file and leave the legacy hook in place, with
            // the content assertions below passing against the wrong path.
            assert_eq!(
                path.file_name(),
                Some(std::ffi::OsStr::new("pre-commit")),
                "install must target .git/hooks/pre-commit for {marker:?}, got: {}",
                path.display()
            );
            assert_eq!(
                std::fs::read_to_string(&path).unwrap(),
                HOOK_SCRIPT,
                "the hook carrying {marker:?} must be replaced with HOOK_SCRIPT"
            );
            assert!(String::from_utf8(buf).unwrap().contains("Updating outdated"));
        }
    }

    /// Every field of `HOOK_CONFIG` names the commit hook: the filename, the
    /// skip env var, the crate name, the script, and help text that mentions
    /// commits and not pushes. `HOOK_CONFIG` is the macro-argument bundle a
    /// copy-paste of this crate into a new hook gets wrong, so each argument
    /// is pinned here — the mirror of `run-before-push`'s test of the same
    /// name.
    #[test]
    fn hook_config_pins_every_macro_argument() {
        assert_eq!(HOOK_CONFIG.hook_filename, "pre-commit");
        assert_eq!(HOOK_CONFIG.skip_env_var, SKIP_ENV_VAR);
        assert_eq!(HOOK_CONFIG.name, NAME);
        assert_eq!(HOOK_CONFIG.hook_script, HOOK_SCRIPT);

        assert!(
            !HOOK_CONFIG.command_help.is_empty(),
            "expected non-empty: {:?}",
            HOOK_CONFIG.command_help
        );
        let help = HOOK_CONFIG.command_help.to_ascii_lowercase();
        assert!(
            help.contains("commit") && !help.contains("push"),
            "command_help must describe the commit hook, got: {}",
            HOOK_CONFIG.command_help
        );
    }

    /// Every entry in `legacy_markers` refers to a commit hook, and the
    /// marker this crate's own hook execs is among them. A list carrying
    /// push-hook markers would make `install` refuse to upgrade a real legacy
    /// pre-commit hook, or claim an unrelated one.
    #[test]
    fn hook_config_legacy_markers_only_match_commit_hooks() {
        assert!(
            !HOOK_CONFIG.legacy_markers.is_empty(),
            "expected non-empty: {:?}",
            HOOK_CONFIG.legacy_markers
        );
        for marker in HOOK_CONFIG.legacy_markers {
            assert!(
                marker.contains("commit") && !marker.contains("push"),
                "legacy marker must refer to a commit hook, got: {marker}"
            );
        }
        let own_marker = format!("ops {NAME}");
        assert!(
            HOOK_CONFIG.legacy_markers.contains(&own_marker.as_str()),
            "the marker this crate's hook execs must stay upgradable, got: {:?}",
            HOOK_CONFIG.legacy_markers
        );
    }

    // -- has_staged_files --
    //
    // The shared probe's behaviour (staged change kinds, non-repo, stderr
    // handling, timeouts) is tested in `ops-hook-common`, next to it. These
    // tests pin only the four lines `has_staged_files` itself contributes:
    // the `current_dir()` lookup, the env-driven timeout reaching the probe,
    // the hardcoded `"git"`, and the `anyhow` conversion.

    fn git(dir: &Path, args: &[&str]) {
        git_fixture(dir, args).unwrap_or_else(|e| panic!("{e}"));
    }

    fn init_repo(dir: &Path) {
        git(dir, &["init", "-q", "-b", "main"]);
        git(dir, &["config", "user.email", "test@example.com"]);
        git(dir, &["config", "user.name", "Test"]);
    }

    /// Write an executable fake `git` script into `dir` and return its path.
    #[cfg(unix)]
    fn write_fake_git(dir: &Path, name: &str, script: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, script).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// Run `probe`, retrying while the exec of a freshly written fake `git`
    /// loses the `ETXTBSY` race: any thread that forks between the write and
    /// the `execve` hands the child an inherited write fd. Sleep-based, so
    /// the worst case is 50 x 20 ms = 1 s of added runtime, paid only when
    /// the race fires.
    #[cfg(unix)]
    fn retry_anyhow_while_text_file_busy(
        mut probe: impl FnMut() -> anyhow::Result<bool>,
    ) -> anyhow::Result<bool> {
        for _ in 0..50 {
            match probe() {
                Err(e) => match e.downcast_ref::<HasStagedFilesError>() {
                    Some(HasStagedFilesError::Spawn { source, .. })
                        if source.kind() == std::io::ErrorKind::ExecutableFileBusy =>
                    {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    _ => return Err(e),
                },
                ok => return ok,
            }
        }
        probe()
    }

    /// The shipped predicate reads the process working directory.
    #[test]
    #[serial_test::serial]
    fn has_staged_files_reads_the_process_working_directory() {
        let _env = hermetic_git_env();
        let _timeout = EnvGuard::remove(TIMEOUT_ENV_VAR);
        let dir = tempfile::tempdir().expect("tempdir");
        init_repo(dir.path());
        let _cwd = CwdGuard::new(dir.path()).expect("CwdGuard");

        assert!(
            !has_staged_files().expect("empty index probe"),
            "an empty index must report no staged work"
        );

        std::fs::write("a.txt", "hi").unwrap();
        git(dir.path(), &["add", "a.txt"]);

        assert!(
            has_staged_files().expect("staged index probe"),
            "a staged file in the cwd repo must report staged work"
        );
    }

    /// The env override is not merely parsed — it is the timeout the probe
    /// actually applies. The assertion reads the timeout back out of the
    /// error rather than timing the call, so it pins the value without
    /// depending on machine speed.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn has_staged_files_applies_the_env_timeout() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Named `git`, because `has_staged_files` hardcodes that program name.
        write_fake_git(dir.path(), "git", "#!/bin/sh\nsleep 30\n");

        // `dir` first so the fake `git` shadows the real one; the system bin
        // directories follow because the fake script itself needs `sleep`.
        let _path = EnvGuard::set("PATH", format!("{}:/usr/bin:/bin", dir.path().display()));
        let _timeout = EnvGuard::set(TIMEOUT_ENV_VAR, "1");
        let _cwd = CwdGuard::new(dir.path()).expect("CwdGuard");

        let err = retry_anyhow_while_text_file_busy(has_staged_files)
            .expect_err("a hanging git must time out");
        let msg = format!("{err:#}");

        assert!(
            msg.contains("timed out after 1s"),
            "the env timeout must be the one applied, got: {msg}"
        );
        assert!(
            !msg.contains(&format!("{DEFAULT_GIT_TIMEOUT:?}")),
            "the default timeout must not win over the env override, got: {msg}"
        );
    }

    /// The typed probe error survives the `anyhow` conversion, so the
    /// chain the CLI prints still names what actually failed.
    #[test]
    #[serial_test::serial]
    fn has_staged_files_error_reaches_the_caller_as_an_anyhow_chain() {
        let _env = hermetic_git_env();
        let _timeout = EnvGuard::remove(TIMEOUT_ENV_VAR);
        let dir = tempfile::tempdir().expect("tempdir");
        // Stop git's discovery at the tempdir, so a repository enclosing the
        // temp root cannot turn this into an in-repo probe.
        let _ceiling = EnvGuard::set("GIT_CEILING_DIRECTORIES", dir.path());
        let _cwd = CwdGuard::new(dir.path()).expect("CwdGuard");

        let err = has_staged_files().expect_err("a non-repo cwd must fail");
        assert!(
            err.downcast_ref::<HasStagedFilesError>().is_some(),
            "the typed probe error must survive into the anyhow chain: {err:#}"
        );
        let msg = format!("{err:#}");
        assert!(msg.contains("diff --cached"), "the chain must name the probe, got: {msg}");
    }

    // -- Extension metadata --

    /// Pin the public identifier against external sources of truth.
    #[test]
    fn extension_constants() {
        assert!(
            HOOK_SCRIPT.contains(&format!("ops {NAME}")),
            "HOOK_SCRIPT must dispatch to `ops {NAME}`, got: {HOOK_SCRIPT}"
        );
        assert_eq!(SHORTNAME, NAME, "shortname must track NAME");
        assert!(
            NAME.chars().all(|c| c.is_ascii_lowercase() || c == '-')
                && NAME.starts_with(|c: char| c.is_ascii_lowercase()),
            "NAME must be kebab-case, got: {NAME}"
        );
        assert!(!DESCRIPTION.is_empty(), "expected non-empty: {DESCRIPTION:?}");
    }

    // -- git_timeout_from_env --

    // The parser cases (zero, unparseable, unset) are tested once, in
    // `ops_hook_common::git_state`. Here only the wiring this crate adds:
    // `TIMEOUT_ENV_VAR` is the variable read and `MAX_GIT_TIMEOUT_SECS` the
    // ceiling (`has_staged_files_applies_the_env_timeout` pins the former).

    /// An overlarge value is clamped, not honoured.
    #[test]
    #[serial_test::serial]
    fn git_timeout_from_env_clamps_to_ceiling() {
        let _guard = EnvGuard::set(TIMEOUT_ENV_VAR, "999999999");
        assert_eq!(git_timeout_from_env(), Some(Duration::from_secs(MAX_GIT_TIMEOUT_SECS)));
    }

    /// Capture the WARN emission so a future refactor that drops the
    /// diagnostic while preserving the clamp does not pass silently.
    mod clamp_log_emission {
        use super::*;
        use ops_core::test_utils::capture_warn as capture;

        #[test]
        #[serial_test::serial]
        fn clamps_to_ceiling_emits_warn() {
            let _guard = EnvGuard::set(TIMEOUT_ENV_VAR, "999999999");
            let logs = capture(|| {
                let _ = git_timeout_from_env();
            });
            assert!(logs.contains("WARN"), "expected WARN level, got: {logs}");
            assert!(logs.contains(TIMEOUT_ENV_VAR), "missing env field: {logs}");
            assert!(logs.contains("requested_secs"), "missing requested_secs field: {logs}");
            assert!(logs.contains("ceiling_secs"), "missing ceiling_secs field: {logs}");
            assert_eq!(
                logs.matches("clamping to upper bound").count(),
                1,
                "expected exactly one clamp warn, got: {logs}"
            );
        }

        #[test]
        #[serial_test::serial]
        fn at_ceiling_emits_no_warn() {
            let value = MAX_GIT_TIMEOUT_SECS.to_string();
            let _guard = EnvGuard::set(TIMEOUT_ENV_VAR, &value);
            let logs = capture(|| {
                let _ = git_timeout_from_env();
            });
            assert!(
                !logs.contains("clamping to upper bound"),
                "no clamp warn expected at the boundary, got: {logs}"
            );
        }
    }
}
