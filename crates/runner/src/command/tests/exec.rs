//! Tests for exec / `run_exec` / unit-level helpers.

use super::*;

#[tokio::test]
async fn run_plan_echo_success() {
    let mut runner = test_runner(HashMap::new());
    runner.register_commands(vec![("echo_hi".into(), CommandSpec::Exec(echo_cmd("hi")))]);
    let mut events = Vec::new();
    let results = runner
        .run_plan(&["echo_hi".into()], true, &mut |e| events.push(e))
        .await;
    assert!(results.iter().all(|r| r.success));
    events.assert_has_event(
        |e| matches!(e, RunnerEvent::StepFinished { .. }),
        "should have StepFinished event",
    );
    events.assert_has_event(
        |e| matches!(e, RunnerEvent::RunFinished { success: true, .. }),
        "should have RunFinished with success=true",
    );

    let result = &results[0];
    assert_eq!(result.id, "echo_hi");
    assert!(result.success);
    // TASK-1664: asserted at nanosecond granularity, not milliseconds. The
    // previous `as_millis() > 0` required the step to be *at least a
    // millisecond slow*, which is not a property worth having: `echo hi` on a
    // fast runner completes in under 1ms, `as_millis()` truncates to 0, and the
    // test fails for being quick. Observed failing on CI while passing locally.
    //
    // The invariant actually worth pinning is that the duration was measured at
    // all rather than left at its default.
    assert!(
        result.duration.as_nanos() > 0,
        "duration should be measured, got {:?}",
        result.duration
    );
}

/// TQ-001: Uses 3s sleep with 1s timeout for reliable timing under CI load.
/// Previous 10s/1s was too slow; this provides 3x safety margin while staying fast.
///
/// # Timing Safety Margin
///
/// The 3s/1s ratio provides 3x safety margin for CI environments:
/// - Sleep duration: 3 seconds (command should run for 3s if not timed out)
/// - Timeout: 1 second (command is killed after 1s)
/// - Safety margin: 3x (timeout is 1/3 of sleep)
///
/// This is sufficient for most CI environments but may fail under extreme load.
/// Alternative approaches (mock time, deterministic test doubles) were considered
/// but would add significant complexity for a single test.
#[tokio::test]
async fn run_exec_timeout() {
    let runner = test_runner(HashMap::new());
    let mut spec = sleep_cmd(3);
    spec.timeout_secs = Some(1);
    let spec = Arc::new(spec);
    let mut events = Vec::new();
    let result = runner
        .run_exec("sleep_cmd", &spec, &mut |e| events.push(e))
        .await;
    assert!(!result.success);
    assert!(result
        .message
        .as_ref()
        .is_some_and(|m| m.contains("timed out")));
    assert!(events
        .iter()
        .any(|e| matches!(e, RunnerEvent::StepFailed { .. })));
}

#[tokio::test]
async fn run_unknown_command_returns_error() {
    let runner = test_runner(HashMap::new());
    let result = runner.run("nonexistent", &mut |_| {}).await;
    assert!(result.is_err());
}

mod exec_unit_tests {
    use super::*;
    use crate::command::exec::{
        build_step_result, emit_output_events, emit_step_completion, execute_with_timeout,
    };
    use crate::command::results::CommandOutput;

    fn emit(id: &str, stdout: &str, stderr: &str, sink: &mut impl FnMut(RunnerEvent)) {
        emit_output_events(id, &Arc::from(stdout), &Arc::from(stderr), sink);
    }

    #[tokio::test]
    async fn execute_with_timeout_no_timeout_succeeds() {
        let mut cmd = tokio::process::Command::new("echo");
        cmd.arg("test");
        let result = execute_with_timeout(cmd, None).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());
    }

    #[tokio::test]
    async fn execute_with_timeout_with_timeout_returns_output() {
        let mut cmd = tokio::process::Command::new("echo");
        cmd.arg("test");
        let result = execute_with_timeout(cmd, Some(Duration::from_secs(5))).await;
        assert!(result.is_ok());
    }

    /// CONC-9 / TASK-1064: when the surrounding parallel task is aborted while
    /// the child is wedged (e.g. `fail_fast` tripped after a sibling failed and
    /// the current child is stuck on `child.wait()`), the per-stream pipe
    /// drains spawned inside `spawn_capped` must die with the parent rather
    /// than detach and keep reading until the wedged child finally exits.
    /// Regression test: spawn `spawn_capped` against a `sleep 60` child inside
    /// a `JoinSet`, `abort_all`, and assert wall-clock for the abort path is
    /// well under the child's runtime. With the pre-fix bare `tokio::spawn`,
    /// the drain tasks would survive the abort and the test reaches the
    /// timeout asserting drain-task survival.
    #[tokio::test]
    async fn spawn_capped_drains_die_with_parent_joinset_abort() {
        use tokio::time::{timeout, Duration};
        let mut outer: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
        outer.spawn(async move {
            let mut cmd = tokio::process::Command::new("sleep");
            cmd.arg("60");
            // Kill on drop so the child is reaped even if abort interrupts us
            // mid-spawn — keeps the test from leaking processes.
            cmd.kill_on_drop(true);
            let _ = crate::command::exec::spawn_capped_for_test(&mut cmd, 1024).await;
        });
        // Yield once so the spawned task actually starts the child + drains.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let start = std::time::Instant::now();
        outer.abort_all();
        // Drain the JoinSet — every aborted handle yields a JoinError(cancelled).
        timeout(Duration::from_secs(2), async {
            while outer.join_next().await.is_some() {}
        })
        .await
        .expect("aborting parent JoinSet must release spawn_capped within 2s");
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_secs(2),
            "expected bounded abort wall-clock, got {elapsed:?}"
        );
    }

    /// PERF-1 / TASK-0764: a child that floods stdout past the configured cap
    /// must not buffer the full output. `spawn_capped` reads up to `cap` and
    /// drains the rest into a sink — the resulting `CommandOutput.stdout` is
    /// bounded near `cap` (head + the marker line) regardless of how much the
    /// child actually wrote.
    #[tokio::test]
    async fn spawn_capped_bounds_collected_bytes_near_cap() {
        let cap: usize = 1024;
        let total: usize = 256 * 1024;
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c")
            .arg(format!("head -c {total} /dev/zero | tr '\\0' a"));
        let output = crate::command::exec::spawn_capped_for_test(&mut cmd, cap)
            .await
            .expect("spawn_capped");
        assert!(output.success);
        // Head + a single short marker line; bound generously to absorb
        // platform-dependent marker punctuation.
        assert!(
            output.stdout.len() < cap + 256,
            "expected bounded stdout near cap={cap}, got {} bytes",
            output.stdout.len()
        );
        assert!(
            output.stdout.contains("[ops] output truncated"),
            "marker line missing from streamed output"
        );
    }

    /// ASYNC-6 / TASK-1918: the captured path must hand the child a *null*
    /// stdin, so a step that reads stdin terminates immediately instead of
    /// blocking on `read(0)` forever (`timeout_secs` is opt-in, so nothing
    /// else bounds it).
    ///
    /// The command is deliberately pre-configured with `Stdio::piped()`
    /// stdin — a pipe whose write end this process holds and never writes
    /// to, i.e. a stdin that never yields EOF on its own. `spawn_capped`
    /// must override it. Pre-fix, `spawn_capped` set only stdout/stderr, the
    /// piped stdin survived, and `read x` parked forever; the outer
    /// `timeout` below is the assertion, not a convenience.
    #[cfg(unix)]
    #[tokio::test]
    async fn spawn_capped_gives_child_null_stdin_so_reader_cannot_hang() {
        use tokio::time::{timeout, Duration};
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg("read x; echo \"read returned $?\"");
        cmd.stdin(std::process::Stdio::piped());
        cmd.kill_on_drop(true);
        let output = timeout(
            Duration::from_secs(5),
            crate::command::exec::spawn_capped_for_test(&mut cmd, 1024),
        )
        .await
        .expect("captured step must not block on stdin")
        .expect("spawn_capped");
        // `read` hits EOF straight away and reports failure; the step
        // completes rather than wedging.
        assert!(
            output.stdout.contains("read returned"),
            "child should have run past its stdin read, got {:?}",
            output.stdout
        );
    }

    /// CONC-9 / TASK-1919: a timed-out step must take its whole process
    /// *tree* with it, not just the direct child.
    ///
    /// The step forks a long-lived grandchild and records its pid, then
    /// blocks past the 1s timeout. Pre-fix, `kill_on_drop` `SIGKILL`ed the
    /// `sh` leader alone and the `sleep` survived the runner that spawned
    /// it — the "cancelled step leaves a compiler fleet pinning the machine"
    /// failure. With the child spawned as its own process-group leader, the
    /// cancellation path signals the group and the grandchild goes too.
    #[cfg(unix)]
    #[tokio::test]
    async fn timed_out_step_kills_the_whole_process_group() {
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("grandchild.pid");
        let script = format!(
            "sleep 30 & echo $! > {}; sleep 30",
            pid_file.to_str().unwrap()
        );
        let runner = test_runner(HashMap::new());
        let mut spec = exec_spec("sh", &["-c", &script]);
        spec.timeout_secs = Some(1);
        let spec = Arc::new(spec);

        let started = std::time::Instant::now();
        let result = runner.run_exec("tree", &spec, &mut |_| {}).await;
        assert!(!result.success, "step should have timed out");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timed-out step should return promptly, took {:?}",
            started.elapsed()
        );

        let pid: i32 = std::fs::read_to_string(&pid_file)
            .expect("grandchild pid file")
            .trim()
            .parse()
            .expect("grandchild pid");
        // The group teardown is SIGTERM-then-SIGKILL, and the signal is
        // delivered from a destructor on the cancellation path, so poll
        // rather than assuming the grandchild is already reaped.
        let mut alive = true;
        for _ in 0..100 {
            // SAFETY: `kill` with signal 0 performs the permission and
            // existence check only — it delivers nothing and touches no
            // memory.
            if unsafe { libc::kill(pid, 0) } != 0 {
                alive = false;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if alive {
            // Do not leak a 30s sleep into the rest of the test run.
            // SAFETY: as above; `pid` is the recorded grandchild.
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
        assert!(
            !alive,
            "grandchild {pid} survived cancellation of its parent step"
        );
    }

    /// CONC-9 / TASK-1919: `child.wait()` returning does not close the
    /// capture pipes. Here the leader exits at once while a backgrounded
    /// grandchild keeps the inherited write end open, so `read_capped` never
    /// sees EOF. Pre-fix the step hung forever (and, with `timeout_secs`
    /// unset by default, hung the whole plan on a child that had already
    /// exited). The post-exit drain is now bounded: the deadline expires,
    /// the process group is killed, the pipes close, and the step completes
    /// with the output the leader did produce.
    #[cfg(unix)]
    #[tokio::test]
    async fn post_exit_drain_is_bounded_when_a_grandchild_holds_the_pipe() {
        use tokio::time::{timeout, Duration as TokioDuration};
        let mut cmd = tokio::process::Command::new("sh");
        // The grandchild must outlive the outer timeout by a wide margin: if
        // its sleep were the same magnitude, the test would also pass when the
        // drain is unbounded and the grandchild simply exits on its own.
        cmd.arg("-c").arg("sleep 300 & echo done");
        cmd.kill_on_drop(true);
        let output = timeout(
            TokioDuration::from_secs(30),
            crate::command::exec::spawn_capped_for_test(&mut cmd, 1024),
        )
        .await
        .expect("post-exit drain must be bounded")
        .expect("spawn_capped");
        assert!(output.success);
        assert!(
            output.stdout.contains("done"),
            "the leader's own output must survive the bounded drain, got {:?}",
            output.stdout
        );
    }

    #[test]
    fn emit_step_completion_success() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let output = CommandOutput {
            success: true,
            stdout: "out".to_string(),
            stderr: String::new(),
            status_message: "exit status: 0".to_string(),
        };
        emit_step_completion(
            "test",
            Duration::from_millis(100),
            &output,
            Some("echo test".to_string()),
            &mut |e| events.push(e),
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            RunnerEvent::StepFinished { id, duration_secs, .. }
            if id == "test" && *duration_secs > 0.0
        ));
    }

    #[test]
    fn emit_step_completion_failure() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let output = CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: "error".to_string(),
            status_message: "exit status: 1".to_string(),
        };
        emit_step_completion(
            "fail_cmd",
            Duration::from_millis(50),
            &output,
            Some("false".to_string()),
            &mut |e| events.push(e),
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            RunnerEvent::StepFailed { id, message, .. }
            if id == "fail_cmd" && message.contains("exit status")
        ));
    }

    #[test]
    fn build_step_result_from_success_output() {
        let output = CommandOutput {
            success: true,
            stdout: "stdout content".to_string(),
            stderr: String::new(),
            status_message: "exit status: 0".to_string(),
        };
        let result = build_step_result("cmd", Duration::from_millis(200), output);
        assert_eq!(result.id, "cmd");
        assert!(result.success);
        assert_eq!(result.duration, Duration::from_millis(200));
        assert_eq!(result.stdout, "stdout content");
        assert!(result.message.is_none());
    }

    #[test]
    fn build_step_result_from_failure_output() {
        let output = CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: "stderr content".to_string(),
            status_message: "exit status: 1".to_string(),
        };
        let result = build_step_result("fail", Duration::from_millis(100), output);
        assert_eq!(result.id, "fail");
        assert!(!result.success);
        assert_eq!(result.stderr, "stderr content");
        assert_eq!(result.message, Some("exit status: 1".to_string()));
    }

    #[test]
    fn emit_output_events_empty_input() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        emit("test", "", "", &mut |e| events.push(e));
        assert!(events.is_empty(), "empty input should produce no events");
    }

    #[test]
    fn emit_output_events_crlf_handling() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        emit("test", "line1\r\nline2\r\n", "err\r\n", &mut |e| {
            events.push(e);
        });

        let stdout_events = events
            .iter()
            .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: false, .. }))
            .count();
        let stderr_events = events
            .iter()
            .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: true, .. }))
            .count();

        assert_eq!(stdout_events, 2);
        assert_eq!(stderr_events, 1);
    }

    /// PERF-3 / TASK-0732: a 10k-line stderr step must reuse a single
    /// shared `Arc<str>` across every emitted `StepOutput` event so the
    /// per-line cost is an `Arc::clone` (atomic refcount inc) rather than
    /// a fresh heap allocation. We can't observe allocator counts from
    /// stable Rust, so the assertion here pins the invariant that every
    /// emitted line points at the *same* underlying buffer pointer — if a
    /// future refactor reverts to per-line String allocations, line addresses
    /// diverge across events and this fails.
    ///
    /// TEST-15 / TASK-1664: a second, wall-clock assertion was removed; see
    /// the note at the end of the body.
    #[test]
    fn emit_output_events_shares_buffer_across_lines() {
        // Build a 10k-line stderr payload deterministically.
        let mut payload = String::with_capacity(10_000 * 32);
        for i in 0..10_000 {
            use std::fmt::Write as _;
            writeln!(payload, "stderr line {i:05} with some padding text").unwrap();
        }

        let mut events: Vec<RunnerEvent> = Vec::with_capacity(10_001);
        emit("noisy", "", &payload, &mut |e| events.push(e));

        let lines: Vec<&crate::command::OutputLine> = events
            .iter()
            .filter_map(|e| match e {
                RunnerEvent::StepOutput {
                    line, stderr: true, ..
                } => Some(line),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 10_000, "one event per line");

        // Every line must point at *the same* underlying str buffer — the
        // canary that verifies Arc-sharing rather than per-line allocs.
        let first_addr = lines[0].as_str().as_ptr().addr();
        let buf_lo = first_addr.saturating_sub(payload.len());
        let buf_hi = first_addr.saturating_add(payload.len());
        for (i, line) in lines.iter().enumerate() {
            let addr = line.as_str().as_ptr().addr();
            assert!(
                addr >= buf_lo && addr <= buf_hi,
                "line {i} points outside the shared buffer (addr {addr:x}, range {buf_lo:x}..{buf_hi:x})"
            );
        }

        // TEST-15 / TASK-1664: the wall-clock assertion that used to sit here
        // (10k lines under 250ms) has been removed. Its own comment conceded
        // it did not detect the regression — "the pre-fix per-line String
        // allocation passed too, so this is a sanity floor, not a precision
        // regression detector" — while being the most reliable failure in the
        // suite once tests run in parallel: it passes in isolation (~0.08s)
        // and fails on every full-suite run, because a 250ms budget in a debug
        // build cannot survive sharing a machine with the other tests.
        //
        // The pointer-identity check above is the real detector, exactly as
        // this test's doc comment describes: per-line allocations would make
        // the line addresses diverge.
    }

    /// PERF-3 / TASK-0838: explicit `Arc::ptr_eq` + `strong_count` assertion.
    /// All emitted lines must share *one* backing `Arc<str>` per stream and
    /// the `strong_count` must reflect (1 buffer + N line events).
    #[test]
    fn emit_output_events_arc_ptr_eq_per_stream() {
        let stdout = "a\nb\nc\n";
        let mut events: Vec<RunnerEvent> = Vec::new();
        emit("t", stdout, "", &mut |e| events.push(e));

        let lines: Vec<&crate::command::OutputLine> = events
            .iter()
            .filter_map(|e| match e {
                RunnerEvent::StepOutput {
                    line,
                    stderr: false,
                    ..
                } => Some(line),
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 3);

        let first = lines[0].buf_arc();
        for line in &lines[1..] {
            assert!(
                std::sync::Arc::ptr_eq(first, line.buf_arc()),
                "all lines in one stream must share one Arc<str>"
            );
        }
        // 3 events, each holding one Arc clone of the buffer; no extra
        // owners exist after emit returns. (We did not retain a separate
        // outer Arc handle in the caller.)
        assert_eq!(
            std::sync::Arc::strong_count(first),
            3,
            "strong_count must equal the number of emitted line events for the stream"
        );
    }

    #[test]
    fn emit_output_events_trailing_newline() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        emit("test", "line1\nline2\n\n", "", &mut |e| events.push(e));

        let stdout_events = events
            .iter()
            .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: false, .. }))
            .count();

        assert_eq!(
            stdout_events, 3,
            "should include empty line from trailing newline"
        );
    }

    #[test]
    fn build_command_includes_env_vars() {
        let mut spec = exec_spec("echo", &["test"]);
        spec.env
            .insert("MY_VAR".to_string(), "my_value".to_string());
        let cmd = build_command(&spec, std::path::Path::new("."), &test_vars()).unwrap();
        let env = cmd.as_std().get_envs().collect::<Vec<_>>();
        assert!(
            env.iter().any(|(k, _)| k.to_string_lossy() == "MY_VAR"),
            "env var should be set"
        );
    }
}

/// TASK-0450 / ERR-1: a non-UTF-8 env value referenced via `${VAR}` in argv
/// or cwd must surface as a `StepFailed` with a user-visible message instead
/// of silently flowing the literal `${VAR}` into the spawned command.
#[cfg(unix)]
#[tokio::test]
#[serial_test::serial]
async fn run_exec_fails_loudly_on_non_utf8_env_var() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let key = "OPS_TEST_RUNNER_NON_UTF8";
    let bad: OsString = OsString::from_vec(vec![0xff, 0xfe, 0xfd]);
    // SAFETY: test-only guard via #[serial] attribute.
    unsafe { std::env::set_var(key, &bad) };

    let runner = test_runner(HashMap::new());
    let spec = Arc::new(exec_spec("echo", &[&format!("${{{key}}}/payload")]));
    let mut events = Vec::new();
    let result = runner
        .run_exec("bad_var", &spec, &mut |e| events.push(e))
        .await;

    // SAFETY: test-only guard via #[serial] attribute.
    unsafe { std::env::remove_var(key) };

    assert!(!result.success, "non-UTF-8 env must fail the step");
    let msg = result.message.as_deref().unwrap_or_default();
    assert!(
        !msg.contains(&format!("${{{key}}}")),
        "user-visible message must not contain the unexpanded literal, got: {msg}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RunnerEvent::StepFailed { .. })),
        "should emit StepFailed for expansion failure"
    );
}

mod error_path_tests {
    use super::*;

    #[tokio::test]
    async fn run_exec_nonexistent_program() {
        let runner = test_runner(HashMap::new());
        let spec = Arc::new(exec_spec("nonexistent_program_xyz123", &[]));
        let mut events = Vec::new();
        let result = runner
            .run_exec("bad_cmd", &spec, &mut |e| events.push(e))
            .await;
        assert!(!result.success, "should fail for nonexistent program");
        assert!(result.message.is_some());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, RunnerEvent::StepFailed { .. })),
            "should emit StepFailed event"
        );
    }

    #[tokio::test]
    async fn run_exec_invalid_cwd() {
        let runner = test_runner(HashMap::new());
        let spec = Arc::new(exec_spec_with_cwd(
            "echo",
            &["test"],
            Some(PathBuf::from("/nonexistent/directory/xyz123")),
        ));
        let result = runner.run_exec("bad_cwd", &spec, &mut |_| {}).await;
        assert!(!result.success, "should fail for invalid cwd");
        assert!(result.message.is_some());
    }

    /// TQ-001: Permission denied error path (non-executable file).
    #[cfg(unix)]
    #[tokio::test]
    async fn run_exec_permission_denied() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("not_executable.sh");
        {
            let mut f = std::fs::File::create(&script).unwrap();
            f.write_all(b"#!/bin/sh\necho hello\n").unwrap();
        }
        let runner = test_runner(HashMap::new());
        let spec = Arc::new(exec_spec(script.to_str().unwrap(), &[]));
        let mut events = Vec::new();
        let result = runner
            .run_exec("perm_denied", &spec, &mut |e| events.push(e))
            .await;
        assert!(!result.success, "should fail for non-executable file");
        assert!(result.message.is_some());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, RunnerEvent::StepFailed { .. })),
            "should emit StepFailed event for permission denied"
        );
    }

    #[test]
    fn list_command_ids_includes_all_commands() {
        let mut commands = HashMap::new();
        commands.insert(
            "build".to_string(),
            CommandSpec::Exec(exec_spec("cargo", &["build"])),
        );
        commands.insert(
            "test".to_string(),
            CommandSpec::Exec(exec_spec("cargo", &["test"])),
        );
        let runner = test_runner(commands);
        let ids = runner.list_command_ids();
        assert!(ids.contains(&"build".into()));
        assert!(ids.contains(&"test".into()));
        assert!(
            ids.len() >= 2,
            "should have at least build and test commands"
        );
    }

    #[test]
    fn list_command_ids_with_extension_commands() {
        let mut runner = test_runner(HashMap::new());
        runner.register_commands(vec![(
            "ext_cmd".into(),
            CommandSpec::Exec(exec_spec("echo", &["ext"])),
        )]);
        let ids = runner.list_command_ids();
        assert!(ids.contains(&"ext_cmd".into()));
    }
}
