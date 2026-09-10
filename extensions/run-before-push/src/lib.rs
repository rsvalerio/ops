//! Run-before-push hook extension: install and manage git pre-push hooks.

use ops_extension::ExtensionType;

/// Extension identifier used to register this crate in the engine's
/// extension registry.
///
/// Doubles as the on-disk spelling the installed hook script execs
/// (`exec ops run-before-push`) and the `.ops.toml` `[commands.<name>]` key
/// `ensure_config_command` writes, so all three agree on one string.
pub const NAME: &str = "run-before-push";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "Setup git pre-push hook to run an ops command of your choice";
/// CLI-facing short name — the subcommand the user types
/// (`ops run-before-push install`).
///
/// Equal to `NAME` in this crate because the registry key and the
/// user-typed command have no reason to differ here; `SHORTNAME` exists as
/// a separate constant so a future rename of either side has a place to
/// land.
pub const SHORTNAME: &str = "run-before-push";

/// Command extension wiring the pre-push install and dispatch surface.
///
/// Registers no data providers and no commands of its own; the CLI reaches
/// this crate's install entry points through `crates/cli/src/pre_hook_cmd.rs`.
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
/// Five properties are load-bearing and covered by tests below:
///
/// 1. **`#!/bin/sh`, not bash** — the body uses nothing bash provides, and a
///    bash dependency breaks the hook on busybox/Alpine images and NixOS
///    shells without bash in scope.
/// 2. **`ops` is probed before it is exec'd** — git hooks fired from GUI
///    clients inherit a truncated PATH, and a bare `command not found` names
///    neither ops nor the fix, so users reach for `git push --no-verify`.
/// 3. **The bypass is honoured before the probe** — the probe's diagnostic
///    advertises `SKIP_OPS_RUN_BEFORE_PUSH` as the escape hatch, so it has
///    to work with `ops` off PATH, the exact situation the diagnostic
///    describes. The bypass-then-probe prologue is shared with the
///    pre-commit hook through [`ops_hook_common::hook_script!`], not
///    copy-pasted.
/// 4. **git's ref-update stream never reaches a spawned command** — git
///    writes one `<local ref> <local oid> <remote ref> <remote oid>` line per
///    ref update to the hook's stdin. The script captures it into
///    [`REF_UPDATES_ENV_VAR`] and redirects `ops` from `/dev/null`, so no
///    configured command can consume it.
/// 5. **The capture is bounded, and a failed or truncated capture fails
///    closed** — `$(...)` yields the empty string on *any* capture failure,
///    and the empty stream means "nothing to push", which skips every
///    configured check downstream; so the capture's exit status is checked
///    and a failed read aborts the hook. And `execve` caps a single
///    environment string at 131072 bytes, so a whole-stream capture would
///    die at exec with `E2BIG` on pushes beyond ~1000 refs; the read is
///    bounded to a byte budget, and a stream that exceeds it is forwarded
///    with [`REFS_TRUNCATED_ENV_VAR`] set, which makes ops run every
///    configured check — never a silent skip, never a bare "Argument list
///    too long".
const HOOK_SCRIPT: &str = ops_hook_common::hook_script! {
    name: "run-before-push",
    hook_filename: "pre-push",
    skip_env_var: "SKIP_OPS_RUN_BEFORE_PUSH",
    tail:
        "# git writes one ref-update line per pushed ref to this hook's stdin. Read a\n",
        "# bounded prefix here and hand ops /dev/null so no configured command can\n",
        "# consume the stream. Two load-bearing details:\n",
        "#\n",
        "# - The capture's exit status is checked because $(...) collapses 'no ref\n",
        "#   updates' and 'could not read the stream' into the same empty string —\n",
        "#   and the empty stream skips every configured check downstream.\n",
        "# - execve caps one environment string at 131072 bytes, so a whole-stream\n",
        "#   capture would die at exec with E2BIG on pushes beyond ~1000 refs. The\n",
        "#   read is bounded to 96000 bytes plus one sentinel byte: a result longer\n",
        "#   than the budget means the stream was truncated, which sets the\n",
        "#   truncation marker below so ops runs every configured check instead of\n",
        "#   classifying a prefix.\n",
        "OPS_PRE_PUSH_REFS=$(head -c 96001)\n",
        "capture_status=$?\n",
        "if [ \"$capture_status\" -ne 0 ]; then\n",
        "    echo \"pre-push: failed to read git's ref-update stream (head exited with status $capture_status); refusing to skip the configured checks.\" >&2\n",
        "    exit 1\n",
        "fi\n",
        "if [ \"${#OPS_PRE_PUSH_REFS}\" -gt 96000 ]; then\n",
        "    OPS_PRE_PUSH_REFS_TRUNCATED=1\n",
        "    echo \"pre-push: ref-update stream exceeds the 96000-byte capture budget; running all configured checks.\" >&2\n",
        "else\n",
        "    OPS_PRE_PUSH_REFS_TRUNCATED=\n",
        "fi\n",
        "export OPS_PRE_PUSH_REFS OPS_PRE_PUSH_REFS_TRUNCATED\n",
        "exec ops run-before-push </dev/null\n",
};

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
///
/// A *present* value always means the stream was actually read. The
/// installed hook checks the capture's exit status and aborts before
/// exec'ing `ops` when it is non-zero, so "no ref updates" (empty string,
/// successful capture) and "capture failed" can never arrive here as the
/// same value — the latter never arrives at all.
///
/// The value is a *bounded prefix* of the stream. The hook reads at most
/// its 96000-byte capture budget, so a push too large to forward whole
/// still reaches ops; a truncation is signalled separately through
/// [`REFS_TRUNCATED_ENV_VAR`], never by quietly clipping the value.
pub const REF_UPDATES_ENV_VAR: &str = "OPS_PRE_PUSH_REFS";

/// Environment variable through which the installed hook signals that the
/// ref-update stream exceeded its capture budget and the value in
/// [`REF_UPDATES_ENV_VAR`] is a truncation.
///
/// Set to `1` only by the hook, only on truncation. [`push_refs`] maps that
/// straight to [`PushRefs::Run`] without consulting the truncated value, so
/// a push too large to classify runs every configured check instead of
/// classifying a prefix that might happen to look delete-only. A push large
/// enough to hit this is a mirror or tag-heavy push; running the checks is
/// the fail-safe direction for a verification gate.
pub const REFS_TRUNCATED_ENV_VAR: &str = "OPS_PRE_PUSH_REFS_TRUNCATED";

/// Upper bound on the ref-update lines parsed from the stream (external
/// input, so it is bounded). A push above this is not worth classifying —
/// run the checks.
///
/// Sized to stay within reach of the hook's byte budget. At `1_000`, a
/// stream of short ref lines (~90 bytes each) fits the hook's 96000-byte
/// capture budget and still trips this line bound, so both bounds are
/// live; a larger line bound could never be reached, because a stream
/// long enough to hit it would already exceed the capture budget.
const MAX_REF_UPDATE_LINES: usize = 1_000;

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
/// object ids (shape is validated before acting on it). Anything else — a
/// malformed line, or more than [`MAX_REF_UPDATE_LINES`] of them — yields
/// [`PushRefs::Run`]: the classifier only ever *skips* work on input it
/// fully understood, so a parser gap can never silently disable the gate.
/// The same invariant holds one boundary up: the installed hook aborts
/// rather than forwarding a stream whose capture failed, so
/// [`PushRefs::NothingToPush`] is reachable only from a stream that was
/// genuinely read as empty.
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
///
/// A set truncation marker short-circuits to [`PushRefs::Run`] — the
/// forwarded value is a prefix, and classifying a prefix of a too-large
/// push could read a delete-only remainder as "delete-only push" and skip
/// the checks.
#[must_use]
pub fn push_refs() -> PushRefs {
    if refs_truncated() {
        return PushRefs::Run;
    }
    classify_ref_updates(std::env::var(REF_UPDATES_ENV_VAR).ok().as_deref())
}

/// True when the installed hook signalled [`REFS_TRUNCATED_ENV_VAR`] = `1`.
fn refs_truncated() -> bool {
    std::env::var(REFS_TRUNCATED_ENV_VAR).is_ok_and(|v| v == "1")
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

    // `EnvGuard` mutates process-global state (environ)
    // and `std::process::Command` snapshots the environment while building a
    // child, so a spawning test running on another harness thread races the
    // serial env mutators — a setenv/getenv data race, not mere flakiness.
    // `serial_test::serial` only serializes its members against *each
    // other*, so every test in this module that spawns a child process
    // (directly or through `run_hook_script`) MUST carry
    // `#[serial_test::serial]`, same key as the mutating tests. A new
    // spawning test that omits the attribute silently opts out of the
    // guarantee.

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

    /// Run `HOOK_SCRIPT` under `/bin/sh` from the scratch directory `path`,
    /// with `stdin`, the caller-chosen `path_value` (the child's `PATH`) and
    /// any extra `envs` under test.
    ///
    /// `path_value` is passed through verbatim — the caller decides what the
    /// child can resolve, because the two situations under test need opposite
    /// PATHs:
    ///
    /// - The missing-ops guard and the bypass fire before any external
    ///   command (`case`, `command -v` and `echo` are `sh` builtins), so
    ///   their tests pass only an empty tempdir: `command -v ops` then fails
    ///   unconditionally, on every machine, and the fall-through can never
    ///   exec a real `ops`.
    /// - The dispatch tests need `head` (the hook's bounded capture) and
    ///   `cat` (their fake `ops` bodies), so they pass their tempdir *plus*
    ///   the system bin directories — tempdir first, so the fake `ops`
    ///   shadows any real one and the `exec` line resolves only the fake.
    #[cfg(unix)]
    fn run_hook_script(
        path: &std::path::Path,
        path_value: &str,
        stdin: &str,
        envs: &[(&str, &str)],
    ) -> (std::process::ExitStatus, String, String) {
        use std::io::Write as _;

        let script = path.join("pre-push");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();

        let mut command = std::process::Command::new("/bin/sh");
        command
            .arg(&script)
            .env("PATH", path_value)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        for (key, value) in envs {
            command.env(key, value);
        }
        let mut child = command.spawn().unwrap();
        // The bounded capture stops reading once its
        // byte budget is met, so an oversized stream legitimately closes
        // the pipe under the writer — only BrokenPipe is tolerated.
        if let Err(e) = child.stdin.take().unwrap().write_all(stdin.as_bytes()) {
            assert_eq!(
                e.kind(),
                std::io::ErrorKind::BrokenPipe,
                "unexpected stdin write error"
            );
        }
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

    /// The script must not depend on bash being installed.
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

    /// `ops` — and therefore every command it spawns — must be handed
    /// `/dev/null`, never git's ref-update pipe.
    #[test]
    fn hook_script_redirects_ops_stdin_from_dev_null() {
        assert!(HOOK_SCRIPT.contains("exec ops run-before-push </dev/null"));
        assert!(HOOK_SCRIPT.contains(REF_UPDATES_ENV_VAR));
    }

    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
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

    /// A command reading stdin sees EOF, not git's ref lines, and the ref
    /// lines arrive through the environment instead.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn hook_script_hands_ops_an_empty_stdin_and_forwards_refs_via_env() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_ops(
            dir.path(),
            "#!/bin/sh\nprintf 'stdin=[%s]\\n' \"$(cat)\"\nprintf 'refs=[%s]\\n' \"$OPS_PRE_PUSH_REFS\"\n",
        );

        let line = format!("refs/heads/main {SHA1_A} refs/heads/main {SHA1_B}");
        // The fake ops shadows any real one (tempdir first); `head` and `cat`
        // come from the system bin dirs behind it.
        let path_value = format!("{}:/usr/bin:/bin", dir.path().display());
        let (status, stdout, stderr) =
            run_hook_script(dir.path(), &path_value, &format!("{line}\n"), &[]);

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

    /// A missing `ops` fails closed with a message that names the binary,
    /// the hook and the escape hatch. PATH holds only the empty tempdir, so
    /// the probe fails unconditionally on every machine — no real `ops` can
    /// be found or exec'd, and the guard below is always the code under test.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn hook_script_fails_closed_with_diagnostic_when_ops_is_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path_value = dir.path().display().to_string();
        let (status, _stdout, stderr) = run_hook_script(dir.path(), &path_value, "", &[]);

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

    /// When the ref-update capture fails (here: `head` unavailable on a
    /// truncated PATH, the degraded-PATH shape of a GUI git client or
    /// minimal container), the hook must not exit 0 with the checks
    /// skipped — `$(...)` yields the empty string on any capture failure,
    /// and the empty stream means "nothing to push", which skips every
    /// configured command downstream. `ops` is made resolvable and marked
    /// so the test can prove it was never reached.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn hook_script_fails_closed_when_the_ref_capture_fails() {
        use std::io::Write as _;

        let dir = tempfile::tempdir().expect("tempdir");
        fake_ops(dir.path(), "#!/bin/sh\necho OPS-WAS-REACHED\n");
        let script = dir.path().join("pre-push");
        std::fs::write(&script, HOOK_SCRIPT).unwrap();

        // PATH holds the fake ops (so the probe passes) but no `head` (so
        // the capture fails) — exactly the degraded-PATH shape of a GUI git
        // client or minimal container.
        let mut child = std::process::Command::new("/bin/sh")
            .arg(&script)
            .env("PATH", dir.path())
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"refs/heads/main 1111 refs/heads/main 2222\n")
            .unwrap();
        let out = child.wait_with_output().unwrap();

        assert_ne!(
            out.status.code(),
            Some(0),
            "a failed capture must never skip the configured checks"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("pre-push"),
            "the diagnostic must name the hook: {stderr}"
        );
        assert!(
            stderr.contains("ref-update"),
            "the diagnostic must name what could not be read: {stderr}"
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("OPS-WAS-REACHED"),
            "ops must not run on a failed capture, stdout was: {stdout}"
        );
    }

    /// A ref-update stream well past `execve`'s 131072-byte per-string cap
    /// (a mirror or tag-heavy push) must reach `ops` without an `E2BIG` at
    /// exec. The hook bounds the
    /// capture to its 96000-byte budget (plus the one sentinel byte that
    /// proves truncation), says so on stderr, and marks the truncation so
    /// [`push_refs`] runs every configured check instead of classifying a
    /// prefix.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn hook_script_dispatches_an_oversized_stream_without_e2big() {
        let dir = tempfile::tempdir().expect("tempdir");
        fake_ops(
            dir.path(),
            "#!/bin/sh\nprintf 'len=%s truncated=[%s]\\n' \"${#OPS_PRE_PUSH_REFS}\" \"$OPS_PRE_PUSH_REFS_TRUNCATED\"\n",
        );

        // 2000 lines x ~131 bytes ≈ 262 KB — twice the per-string exec cap,
        // which an unbounded capture would hit with E2BIG around n=1200.
        let line = format!(
            "refs/heads/branch-{:04} {SHA1_A} refs/heads/branch-{:04} {SHA1_B}",
            0, 0
        );
        let stream = format!("{line}\n").repeat(2000);

        // The fake ops (printf only) shadows any real one; `head` for the
        // hook's bounded capture comes from the system bin dirs behind it.
        let path_value = format!("{}:/usr/bin:/bin", dir.path().display());
        let (status, stdout, stderr) = run_hook_script(dir.path(), &path_value, &stream, &[]);

        assert_eq!(
            status.code(),
            Some(0),
            "the oversized push must dispatch, not die at exec; stderr was: {stderr}"
        );
        assert!(
            stderr.contains("capture budget"),
            "the truncation must be said out loud, stderr was: {stderr}"
        );
        assert!(
            stdout.contains("truncated=[1]"),
            "ops must see the truncation marker, stdout was: {stdout}"
        );
        let captured_len = stdout
            .lines()
            .find_map(|l| l.strip_prefix("len="))
            .and_then(|l| l.split_whitespace().next())
            .and_then(|n| n.parse::<usize>().ok())
            .unwrap_or_default();
        assert_eq!(
            captured_len, 96_001,
            "the capture must stop at the budget plus the sentinel byte, stdout was: {stdout}"
        );
    }

    /// The missing-ops diagnostic names [`SKIP_ENV_VAR`] as the escape
    /// hatch, so the bypass must fire before the probe that prints it —
    /// otherwise the only advice a stuck user gets is advice that does not
    /// work. PATH holds only the empty tempdir, so the probe fails
    /// unconditionally and the fall-through can never exec a real `ops`.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn hook_script_honours_the_bypass_when_ops_is_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path_value = dir.path().display().to_string();

        for value in ["1", "true", "TRUE", "Yes", "on"] {
            let (status, _stdout, stderr) =
                run_hook_script(dir.path(), &path_value, "", &[(SKIP_ENV_VAR, value)]);
            assert_eq!(
                status.code(),
                Some(0),
                "{value:?} must skip cleanly, stderr was: {stderr}"
            );
        }

        // A value `should_skip` rejects must still reach the probe and fail.
        let (status, _stdout, _stderr) =
            run_hook_script(dir.path(), &path_value, "", &[(SKIP_ENV_VAR, "maybe")]);
        assert_eq!(status.code(), Some(1));
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
                format!("(delete) {ZERO} refs/heads/old {SHA1_A}\n"),
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

    /// The truncation marker short-circuits to `Run` before classification —
    /// a truncated stream is a prefix, and a prefix of a delete-heavy
    /// mirror push could otherwise read as delete-only and skip the checks.
    #[test]
    #[serial_test::serial]
    fn push_refs_runs_when_the_truncation_marker_is_set() {
        let _refs = EnvGuard::set(
            REF_UPDATES_ENV_VAR,
            format!("(delete) {ZERO} refs/heads/old {SHA1_A}\n"),
        );
        assert_eq!(
            {
                let _marker = EnvGuard::remove(REFS_TRUNCATED_ENV_VAR);
                push_refs()
            },
            PushRefs::DeleteOnly,
            "without the marker the stream classifies normally"
        );
        {
            let _marker = EnvGuard::set(REFS_TRUNCATED_ENV_VAR, "1");
            assert_eq!(push_refs(), PushRefs::Run, "marker \"1\" must run");
            assert_eq!(skip_reason(), None, "the checks must not be skipped");
        }
        // Only the literal `1` counts; the hook exports an empty value when
        // it did not truncate, which must not read as truncated.
        let _marker = EnvGuard::set(REFS_TRUNCATED_ENV_VAR, "");
        assert_eq!(
            push_refs(),
            PushRefs::DeleteOnly,
            "an empty marker must not force Run"
        );
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

    /// Pin the public identifier against external sources of truth instead
    /// of comparing the const to a literal copy of itself.
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

    /// `HOOK_CONFIG` is the only thing distinguishing this crate from
    /// `ops-run-before-commit`, and every field of it is a copy-paste
    /// hazard. Pin each one — the identifiers against the literal strings
    /// they must equal, so swapping in the sibling crate's constant fails
    /// here instead of shipping green.
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

    /// A `legacy_markers` list copied from the commit crate would make
    /// `install` refuse to upgrade a real legacy pre-push hook — or claim
    /// an unrelated one.
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
}
