//! Run-before-push hook extension: install and manage git pre-push hooks.

#![cfg_attr(
    test,
    // Test-only policy exception: assertions on known-good fixtures read
    // better as `.unwrap()` than as error-handling ceremony.
    allow(clippy::unwrap_used)
)]

use ops_extension::ExtensionType;

pub const NAME: &str = "run-before-push";
pub const DESCRIPTION: &str = "Setup git pre-push hook to run an ops command of your choice";
pub const SHORTNAME: &str = "run-before-push";

pub struct RunBeforePushExtension;

ops_extension::impl_extension! {
    RunBeforePushExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    data_provider_name: None,
    register_data_providers: |_self, _registry| {},
    factory: RUN_BEFORE_PUSH_FACTORY = |_, _| {
        Some((NAME, Box::new(RunBeforePushExtension)))
    },
}

/// The shell script installed as `.git/hooks/pre-push`.
///
/// Three properties are load-bearing and covered by tests below:
///
/// 1. **`#!/bin/sh`, not bash** — the body uses nothing bash provides, and a
///    bash dependency breaks the hook on busybox/Alpine images and NixOS
///    shells without bash in scope (CL-3 / TASK-1911).
/// 2. **`ops` is probed before it is exec'd** — git hooks fired from GUI
///    clients inherit a truncated PATH, and a bare `command not found` names
///    neither ops nor the fix, so users reach for `git push --no-verify`.
/// 3. **git's ref-update stream never reaches a spawned command** — git
///    writes one `<local ref> <local oid> <remote ref> <remote oid>` line per
///    ref update to the hook's stdin. The script captures it into
///    [`REF_UPDATES_ENV_VAR`] and redirects `ops` from `/dev/null`, so no
///    configured command can consume it (SEC-11 / TASK-1906).
const HOOK_SCRIPT: &str = r#"#!/bin/sh
# Installed by `ops run-before-push install`.
if ! command -v ops >/dev/null 2>&1; then
    echo "pre-push: cannot find the 'ops' binary on PATH (hook: .git/hooks/pre-push)." >&2
    echo "pre-push: add ops to PATH (e.g. ~/.cargo/bin), or bypass with SKIP_OPS_RUN_BEFORE_PUSH=1." >&2
    exit 1
fi
# git writes one ref-update line per pushed ref to this hook's stdin. Capture
# it here and hand ops /dev/null so no configured command can consume it.
OPS_PRE_PUSH_REFS=$(cat)
export OPS_PRE_PUSH_REFS
exec ops run-before-push </dev/null
"#;

/// Environment variable that skips the run-before-push check.
///
/// Recognized values are `1`, `true`, `yes` and `on`, matched
/// case-insensitively; anything else — including the empty string, `0` and
/// `false` — means "do not skip". [`ops_hook_common::should_skip`] is the
/// source of truth for that list; keep this doc in step with it.
pub const SKIP_ENV_VAR: &str = "SKIP_OPS_RUN_BEFORE_PUSH";

/// Environment variable through which the installed hook forwards git's
/// pre-push ref-update stream to `ops`.
///
/// Set (possibly to the empty string) only when `ops run-before-push` was
/// invoked from the hook; absent for a manual invocation, which is how
/// [`classify_ref_updates`] tells "git said nothing is being pushed" apart
/// from "there is no push to reason about".
pub const REF_UPDATES_ENV_VAR: &str = "OPS_PRE_PUSH_REFS";

/// Upper bound on the ref-update lines parsed from the stream (SEC-11: bound
/// external input). A push above this is not worth classifying — run the
/// checks.
const MAX_REF_UPDATE_LINES: usize = 10_000;

/// What git's pre-push ref-update stream says about this push.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushRefs {
    /// At least one ref update writes to the remote, the stream was absent
    /// (manual invocation), or it was malformed: run the configured commands.
    Run,
    /// Every ref update deletes a remote ref — nothing is being sent.
    DeleteOnly,
    /// git reported no ref updates at all.
    NothingToPush,
}

impl PushRefs {
    /// The operator-facing reason to short-circuit, or `None` to run.
    #[must_use]
    pub const fn skip_reason(self) -> Option<&'static str> {
        match self {
            Self::Run => None,
            Self::DeleteOnly => Some("delete-only push"),
            Self::NothingToPush => Some("nothing to push"),
        }
    }
}

/// True for a git object id: 40 (SHA-1) or 64 (SHA-256) hex digits.
fn is_object_id(field: &str) -> bool {
    matches!(field.len(), 40 | 64) && field.bytes().all(|b| b.is_ascii_hexdigit())
}

/// True for the all-zero object id git uses as its "no such object" sentinel.
fn is_zero_object_id(field: &str) -> bool {
    is_object_id(field) && field.bytes().all(|b| b == b'0')
}

/// Classify git's pre-push ref-update stream.
///
/// `stream` is `None` when [`REF_UPDATES_ENV_VAR`] is unset — a manual
/// `ops run-before-push`, where there is no push to reason about.
///
/// Every line must be four whitespace-separated fields with well-formed
/// object ids (SEC-11: validate shape before acting on it). Anything else —
/// a malformed line, or more than [`MAX_REF_UPDATE_LINES`] of them — yields
/// [`PushRefs::Run`]: the classifier only ever *skips* work on input it fully
/// understood, so a parser gap can never silently disable the gate.
#[must_use]
pub fn classify_ref_updates(stream: Option<&str>) -> PushRefs {
    let Some(stream) = stream else {
        return PushRefs::Run;
    };

    let mut updates = 0usize;
    let mut deletions = 0usize;
    for line in stream.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if updates.saturating_add(deletions) >= MAX_REF_UPDATE_LINES {
            return PushRefs::Run;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [local_ref, local_oid, remote_ref, remote_oid] = fields[..] else {
            return PushRefs::Run;
        };
        if local_ref.is_empty()
            || remote_ref.is_empty()
            || !is_object_id(local_oid)
            || !is_object_id(remote_oid)
        {
            return PushRefs::Run;
        }
        // git sends an all-zero *local* oid for a ref being deleted.
        if is_zero_object_id(local_oid) {
            deletions = deletions.saturating_add(1);
        } else {
            updates = updates.saturating_add(1);
        }
    }

    match (updates, deletions) {
        (0, 0) => PushRefs::NothingToPush,
        (0, _) => PushRefs::DeleteOnly,
        _ => PushRefs::Run,
    }
}

/// Classify the ref-update stream the installed hook forwarded through
/// [`REF_UPDATES_ENV_VAR`].
#[must_use]
pub fn push_refs() -> PushRefs {
    classify_ref_updates(std::env::var(REF_UPDATES_ENV_VAR).ok().as_deref())
}

/// Pre-run gate for the `run-before-push` dispatch path: `Some(reason)` to
/// short-circuit the hook with success, `None` to run the configured commands.
#[must_use]
pub fn skip_reason() -> Option<&'static str> {
    push_refs().skip_reason()
}

ops_hook_common::impl_hook_wrappers! {
    name: NAME,
    hook_filename: "pre-push",
    hook_script: HOOK_SCRIPT,
    skip_env_var: SKIP_ENV_VAR,
    legacy_markers: &["ops run-before-push", "ops before-push"],
    command_help: "Run run-before-push checks before pushing",
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_hook_common::test_helpers::EnvGuard;

    const SHA1_A: &str = "1111111111111111111111111111111111111111";
    const SHA1_B: &str = "2222222222222222222222222222222222222222";
    const ZERO: &str = "0000000000000000000000000000000000000000";

    /// Write an executable `ops` stand-in into `dir`, which doubles as the
    /// PATH the hook script under test sees.
    #[cfg(unix)]
    fn fake_ops(dir: &std::path::Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;

        let path = dir.join("ops");
        std::fs::write(&path, body).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    /// Run `HOOK_SCRIPT` under `/bin/sh` with `stdin` and PATH under test.
    ///
    /// PATH is `dir` plus the system bin directories — the script needs
    /// `cat`, and the ambient PATH is deliberately excluded so a developer's
    /// installed `ops` cannot satisfy the "ops missing" case.
    #[cfg(unix)]
    fn run_hook_script(
        path: &std::path::Path,
        stdin: &str,
    ) -> (std::process::ExitStatus, String, String) {
        use std::io::Write as _;

        let script = path.join("pre-push");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();

        let mut child = std::process::Command::new("/bin/sh")
            .arg(&script)
            .env("PATH", format!("{}:/usr/bin:/bin", path.display()))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        (
            out.status,
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    // -- HOOK_SCRIPT --

    #[test]
    fn hook_script_contains_ops_run_before_push() {
        assert!(HOOK_SCRIPT.contains("ops run-before-push"));
    }

    /// CL-3 / TASK-1911: the script must not depend on bash being installed.
    #[test]
    fn hook_script_uses_posix_sh_shebang() {
        assert!(
            HOOK_SCRIPT.starts_with("#!/bin/sh\n"),
            "HOOK_SCRIPT must not depend on bash, got: {HOOK_SCRIPT}"
        );
        assert!(!HOOK_SCRIPT.contains("bash"));
    }

    #[test]
    fn hook_script_guards_missing_ops_binary() {
        assert!(HOOK_SCRIPT.contains("command -v ops"));
        assert!(HOOK_SCRIPT.contains(SKIP_ENV_VAR));
        assert!(HOOK_SCRIPT.contains("exit 1"));
    }

    /// SEC-11 / TASK-1906: `ops` — and therefore every command it spawns —
    /// must be handed `/dev/null`, never git's ref-update pipe.
    #[test]
    fn hook_script_redirects_ops_stdin_from_dev_null() {
        assert!(HOOK_SCRIPT.contains("exec ops run-before-push </dev/null"));
        assert!(HOOK_SCRIPT.contains(REF_UPDATES_ENV_VAR));
    }

    #[cfg(unix)]
    #[test]
    fn hook_script_is_valid_posix_sh() {
        let dir = tempfile::tempdir().expect("tempdir");
        let script = dir.path().join("pre-push");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();
        let status = std::process::Command::new("/bin/sh")
            .arg("-n")
            .arg(&script)
            .status()
            .unwrap();
        assert!(status.success(), "HOOK_SCRIPT must parse under `sh -n`");
    }

    /// TASK-1906 AC#1: a command reading stdin sees EOF, not git's ref lines,
    /// and the ref lines arrive through the environment instead.
    #[cfg(unix)]
    #[test]
    fn hook_script_hands_ops_an_empty_stdin_and_forwards_refs_via_env() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_ops(
            dir.path(),
            "#!/bin/sh\nprintf 'stdin=[%s]\\n' \"$(cat)\"\nprintf 'refs=[%s]\\n' \"$OPS_PRE_PUSH_REFS\"\n",
        );

        let line = format!("refs/heads/main {SHA1_A} refs/heads/main {SHA1_B}");
        let (status, stdout, stderr) = run_hook_script(dir.path(), &format!("{line}\n"));

        assert!(status.success(), "hook failed: {stderr}");
        assert!(
            stdout.contains("stdin=[]"),
            "spawned command must observe EOF on stdin, got: {stdout}"
        );
        assert!(
            stdout.contains(&format!("refs=[{line}]")),
            "ref updates must reach ops through {REF_UPDATES_ENV_VAR}, got: {stdout}"
        );
    }

    /// CL-3 / TASK-1911 AC#2-3: a missing `ops` fails closed with a message
    /// that names the binary, the hook and the escape hatch.
    #[cfg(unix)]
    #[test]
    fn hook_script_fails_closed_with_diagnostic_when_ops_is_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (status, _stdout, stderr) = run_hook_script(dir.path(), "");

        assert!(!status.success(), "a hook that cannot verify must not pass");
        assert!(stderr.contains("ops"), "stderr must name ops: {stderr}");
        assert!(
            stderr.contains(".git/hooks/pre-push"),
            "stderr must name the hook: {stderr}"
        );
        assert!(
            stderr.contains(SKIP_ENV_VAR),
            "stderr must name the bypass: {stderr}"
        );
    }

    // -- classify_ref_updates --

    #[test]
    fn classify_runs_when_stream_is_absent() {
        assert_eq!(classify_ref_updates(None), PushRefs::Run);
    }

    #[test]
    fn classify_reports_nothing_to_push_for_empty_stream() {
        assert_eq!(classify_ref_updates(Some("")), PushRefs::NothingToPush);
        assert_eq!(
            classify_ref_updates(Some("\n  \n")),
            PushRefs::NothingToPush
        );
    }

    #[test]
    fn classify_runs_for_a_normal_ref_update() {
        let line = format!("refs/heads/main {SHA1_A} refs/heads/main {SHA1_B}\n");
        assert_eq!(classify_ref_updates(Some(&line)), PushRefs::Run);
    }

    #[test]
    fn classify_reports_delete_only_when_every_local_oid_is_zero() {
        let stream = format!(
            "(delete) {ZERO} refs/heads/old {SHA1_A}\n(delete) {ZERO} refs/tags/v1 {SHA1_B}\n"
        );
        assert_eq!(classify_ref_updates(Some(&stream)), PushRefs::DeleteOnly);
    }

    #[test]
    fn classify_runs_when_a_delete_is_mixed_with_an_update() {
        let stream = format!(
            "(delete) {ZERO} refs/heads/old {SHA1_A}\nrefs/heads/main {SHA1_A} refs/heads/main {ZERO}\n"
        );
        assert_eq!(classify_ref_updates(Some(&stream)), PushRefs::Run);
    }

    #[test]
    fn classify_runs_for_malformed_lines() {
        // too few fields
        assert_eq!(
            classify_ref_updates(Some(&format!("refs/heads/main {SHA1_A} refs/heads/main\n"))),
            PushRefs::Run
        );
        // too many fields
        assert_eq!(
            classify_ref_updates(Some(&format!(
                "refs/heads/main {SHA1_A} refs/heads/main {SHA1_B} extra\n"
            ))),
            PushRefs::Run
        );
        // oid that is not hex of the right length
        assert_eq!(
            classify_ref_updates(Some(&format!(
                "refs/heads/main zzzz refs/heads/main {SHA1_B}\n"
            ))),
            PushRefs::Run
        );
        // a malformed line must not be masked by a well-formed deletion
        assert_eq!(
            classify_ref_updates(Some(&format!(
                "(delete) {ZERO} refs/heads/old {SHA1_A}\ngarbage\n"
            ))),
            PushRefs::Run
        );
    }

    #[test]
    fn classify_runs_when_the_stream_exceeds_the_line_bound() {
        let line = format!("(delete) {ZERO} refs/heads/old {SHA1_A}\n");
        let stream = line.repeat(MAX_REF_UPDATE_LINES.saturating_add(1));
        assert_eq!(classify_ref_updates(Some(&stream)), PushRefs::Run);
    }

    #[test]
    fn skip_reason_is_none_only_for_run() {
        assert_eq!(PushRefs::Run.skip_reason(), None);
        assert_eq!(PushRefs::DeleteOnly.skip_reason(), Some("delete-only push"));
        assert_eq!(
            PushRefs::NothingToPush.skip_reason(),
            Some("nothing to push")
        );
    }

    #[test]
    #[serial_test::serial]
    fn skip_reason_reads_the_forwarded_env_var() {
        {
            let _guard = EnvGuard::remove(REF_UPDATES_ENV_VAR);
            assert_eq!(push_refs(), PushRefs::Run);
            assert_eq!(skip_reason(), None);
        }
        {
            let _guard = EnvGuard::set(
                REF_UPDATES_ENV_VAR,
                &format!("(delete) {ZERO} refs/heads/old {SHA1_A}\n"),
            );
            assert_eq!(push_refs(), PushRefs::DeleteOnly);
            assert_eq!(skip_reason(), Some("delete-only push"));
        }
        {
            let _guard = EnvGuard::set(REF_UPDATES_ENV_VAR, "");
            assert_eq!(push_refs(), PushRefs::NothingToPush);
            assert_eq!(skip_reason(), Some("nothing to push"));
        }
    }

    // -- should_skip --

    #[test]
    #[serial_test::serial]
    fn should_skip_returns_false_by_default() {
        let _guard = EnvGuard::remove(SKIP_ENV_VAR);
        assert!(!should_skip());
    }

    // -- install_hook: wrapper-specific legacy markers --

    #[test]
    fn install_hook_updates_legacy_before_push_hook() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir_all(git_dir.join("hooks")).unwrap();
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(
            git_dir.join("hooks/pre-push"),
            "#!/bin/sh\nexec ops before-push\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let path = install_hook(&git_dir, &mut buf).expect("install_hook");

        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content, HOOK_SCRIPT);

        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Updating outdated"));
    }

    // -- Extension metadata --

    /// TEST-11 / TASK-0720: pin the public identifier against external
    /// sources of truth instead of comparing the const to a literal copy of
    /// itself. Mirrors the structural checks in run-before-commit so both
    /// crates stay in lockstep.
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
        assert!(!DESCRIPTION.is_empty());
    }

    /// TEST-5 / TASK-1909: `HOOK_CONFIG` is the only thing distinguishing
    /// this crate from `ops-run-before-commit`, and every field of it is a
    /// copy-paste hazard. Pin each one — the identifiers against the literal
    /// strings they must equal, so swapping in the sibling crate's constant
    /// fails here instead of shipping green.
    #[test]
    fn hook_config_pins_every_macro_argument() {
        assert_eq!(HOOK_CONFIG.hook_filename, "pre-push");
        assert_eq!(HOOK_CONFIG.skip_env_var, SKIP_ENV_VAR);
        assert_eq!(SKIP_ENV_VAR, "SKIP_OPS_RUN_BEFORE_PUSH");
        assert_eq!(HOOK_CONFIG.name, NAME);
        assert_eq!(HOOK_CONFIG.hook_script, HOOK_SCRIPT);

        assert!(!HOOK_CONFIG.command_help.is_empty());
        let help = HOOK_CONFIG.command_help.to_ascii_lowercase();
        assert!(
            help.contains("push") && !help.contains("commit"),
            "command_help must describe the push hook, got: {}",
            HOOK_CONFIG.command_help
        );
    }

    /// TEST-5 / TASK-1909: a `legacy_markers` list copied from the commit
    /// crate would make `install` refuse to upgrade a real legacy pre-push
    /// hook — or claim an unrelated one.
    #[test]
    fn hook_config_legacy_markers_only_match_push_hooks() {
        assert!(!HOOK_CONFIG.legacy_markers.is_empty());
        for marker in HOOK_CONFIG.legacy_markers {
            assert!(
                marker.contains("push") && !marker.contains("commit"),
                "legacy marker must refer to a push hook, got: {marker}"
            );
        }
        assert!(
            HOOK_CONFIG
                .legacy_markers
                .iter()
                .any(|m| HOOK_SCRIPT.contains(*m)),
            "the current HOOK_SCRIPT must be covered by a legacy marker"
        );
    }

    /// TEST-5 / TASK-1909: the generated accessor is public surface; exercise
    /// it so it cannot rot into dead code.
    #[test]
    fn hook_config_accessor_returns_the_same_descriptor() {
        let config = hook_config();
        assert_eq!(config.name, HOOK_CONFIG.name);
        assert_eq!(config.hook_filename, HOOK_CONFIG.hook_filename);
        assert_eq!(config.hook_script, HOOK_CONFIG.hook_script);
        assert_eq!(config.skip_env_var, HOOK_CONFIG.skip_env_var);
        assert_eq!(config.legacy_markers, HOOK_CONFIG.legacy_markers);
        assert_eq!(config.command_help, HOOK_CONFIG.command_help);
    }
}
