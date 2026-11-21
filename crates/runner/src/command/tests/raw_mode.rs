//! Tests for raw-mode execution.

use super::*;
use crate::command::exec::exec_command_raw;

#[tokio::test]
async fn exec_command_raw_returns_success_for_true() {
    let spec = Arc::new(true_cmd());
    let result = exec_command_raw("true_cmd", &spec, &test_exec_env()).await;
    assert!(result.success);
    assert!(result.stdout.is_empty(), "raw mode must not capture stdout");
    assert!(result.stderr.is_empty(), "raw mode must not capture stderr");
    assert!(result.message.is_none());
}

#[tokio::test]
async fn exec_command_raw_returns_failure_for_false() {
    let spec = Arc::new(false_cmd());
    let result = exec_command_raw("false_cmd", &spec, &test_exec_env()).await;
    assert!(!result.success);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    assert!(result.message.is_some());
}

#[tokio::test]
async fn run_plan_raw_runs_sequentially_and_collects_results() {
    let mut commands = HashMap::new();
    commands.insert("step_a".to_string(), CommandSpec::Exec(true_cmd()));
    commands.insert("step_b".to_string(), CommandSpec::Exec(true_cmd()));
    let runner = test_runner(commands);

    let results = runner.run_plan_raw(&["step_a".into(), "step_b".into()], true).await;

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.success));
}

#[tokio::test]
async fn run_plan_raw_fail_fast_stops_on_first_failure() {
    let mut commands = HashMap::new();
    commands.insert("fail".to_string(), CommandSpec::Exec(false_cmd()));
    commands.insert("after".to_string(), CommandSpec::Exec(true_cmd()));
    let runner = test_runner(commands);

    let results = runner.run_plan_raw(&["fail".into(), "after".into()], true).await;

    assert_eq!(results.len(), 1, "fail_fast must stop after first failure");
    assert!(!results[0].success);
}

#[tokio::test]
async fn run_plan_raw_unknown_command_returns_failure() {
    let runner = test_runner(HashMap::new());
    let results = runner.run_plan_raw(&["nope".into()], true).await;
    assert_eq!(results.len(), 1);
    assert!(!results[0].success);
    assert!(results[0].message.as_deref().unwrap().contains("unknown"));
}

#[tokio::test]
async fn run_raw_expands_composite_and_runs_leaves() {
    let mut commands = HashMap::new();
    commands.insert("a".to_string(), CommandSpec::Exec(true_cmd()));
    commands.insert("b".to_string(), CommandSpec::Exec(true_cmd()));
    commands.insert("both".to_string(), CommandSpec::Composite(composite_cmd(&["a", "b"])));
    let runner = test_runner(commands);

    let results = runner.run_raw("both").await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.success));
}

/// The raw step's process-tree ownership: a timeout must tear down the tree
/// the step launched, and the terminal choreography (foreground hand-off,
/// Ctrl-C relay, Ctrl-Z stop/resume) must keep behaving like a plain
/// foreground command. The timeout test runs without a terminal (the test
/// harness pipes stdio), i.e. on the own-group / no-tty arm of the
/// classification; the terminal arms need a real controlling terminal and
/// drive one through a pty, with a re-exec'd copy of this test binary as
/// the session leader.
#[cfg(unix)]
mod raw_foreground {
    use super::*;
    use crate::command::exec::exec_command_raw;
    use std::os::fd::AsRawFd;

    /// Poll `kill(pid, 0)` until the process is gone, or `budget` elapses.
    /// Returns whether it is gone.
    fn wait_until_dead(pid: libc::pid_t, budget: Duration) -> bool {
        let started = std::time::Instant::now();
        loop {
            // SAFETY: signal 0 only performs the existence/permission check.
            if unsafe { libc::kill(pid, 0) } != 0 {
                return true;
            }
            if started.elapsed() >= budget {
                return false;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// A raw step that times out must leave no descendants running: the
    /// step forks a grandchild (`sleep`) and blocks in `wait` past its 1s
    /// timeout, so the grandchild outlives the direct child by design. The
    /// raw child leads its own process group and the dropped future tears
    /// the group down, so the recorded grandchild must not survive.
    #[tokio::test]
    async fn raw_timeout_tears_down_the_step_process_tree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pid_file = dir.path().join("grandchild.pid");
        let script =
            format!("sleep 30 & echo $! > {}; wait", pid_file.to_str().expect("utf-8 path"));
        let mut spec = exec_spec("sh", &["-c", &script]);
        spec.timeout_secs = Some(1);
        let spec = Arc::new(spec);

        let started = std::time::Instant::now();
        let result = exec_command_raw("raw_timeout", &spec, &test_exec_env()).await;
        let _ = started;

        assert!(!result.success, "a timed-out raw step must fail");
        let message = result.message.unwrap_or_default();
        assert!(
            message.contains("timed out after 1s"),
            "the failure must be the timeout, got: {message}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "timed-out raw step must return promptly, took {:?}",
            started.elapsed()
        );

        let pid: libc::pid_t = std::fs::read_to_string(&pid_file)
            .expect("grandchild pid file")
            .trim()
            .parse()
            .expect("grandchild pid");
        assert!(
            wait_until_dead(pid, Duration::from_secs(10)),
            "grandchild {pid} survived the timeout of its raw step"
        );
    }

    /// Env var selecting the inner mode; set only on the re-exec'd copy.
    const INNER_MODE: &str = "OPS_RAW_PTY_INNER_MODE";
    /// Env var carrying the pty slave path to the inner process.
    const INNER_PTS: &str = "OPS_RAW_PTY_INNER_PTS";
    /// Env var carrying the file the inner step records its sleep pid in.
    const INNER_PID_FILE: &str = "OPS_RAW_PTY_INNER_PID_FILE";

    /// The inner side of the pty tests, selected by [`INNER_MODE`]: become a
    /// session leader with the pty slave as the controlling terminal, take
    /// the foreground, run one raw step, and mark progress on the terminal.
    /// When the outer test passes `mode` through, the assertions about *our*
    /// fate (dead of SIGINT, never stopped) live in the outer process.
    #[test]
    fn pty_inner_raw_step() {
        let Ok(mode) = std::env::var(INNER_MODE) else { return };
        inner_body(&mode);
    }

    /// Write a marker straight to fd 1 (the pty slave after `dup2`),
    /// bypassing libtest's capture.
    fn marker(text: &str) {
        // SAFETY: write(2) reads only from the provided buffer.
        unsafe {
            libc::write(1, text.as_ptr().cast::<libc::c_void>(), text.len());
        }
    }

    fn inner_body(mode: &str) {
        let pts = std::env::var(INNER_PTS).expect("pts path");
        let pid_file = std::env::var(INNER_PID_FILE).expect("pid file path");
        // SAFETY: setsid(2) takes no arguments and only mutates the caller's
        // session/group membership. The raw syscall returns the previous
        // process-group id on success and -1 on failure, so any non-negative
        // value means the inner is now a session leader.
        assert!(unsafe { libc::setsid() } >= 0, "inner must become a session leader");
        // The first open of the slave after setsid makes it the controlling
        // terminal.
        let slave =
            std::fs::OpenOptions::new().read(true).write(true).open(&pts).expect("open pty slave");
        let slave_fd = slave.as_raw_fd();
        // Block SIGTTOU for the initial tcsetpgrp: this fresh process has
        // none of the runner's dispositions, and the call happens while no
        // foreground group is established yet.
        //
        // SAFETY: the sigset_t values are owned by this frame and only
        // written through the libc calls; dup2/tcsetpgrp/tcgetpgrp/getpgrp
        // take plain integer descriptors and group ids.
        unsafe {
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&raw mut set);
            libc::sigaddset(&raw mut set, libc::SIGTTOU);
            libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, std::ptr::null_mut());
            for fd in 0..3 {
                libc::dup2(slave_fd, fd);
            }
            libc::tcsetpgrp(0, libc::getpgrp());
        }
        assert_eq!(
            unsafe { libc::tcgetpgrp(0) },
            unsafe { libc::getpgrp() },
            "the inner process must hold the pty's foreground"
        );
        marker("INNER-READY\n");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("inner runtime");
        let run = |script: &str, timeout_secs: u64| {
            let mut spec = exec_spec("sh", &["-c", script]);
            spec.timeout_secs = Some(timeout_secs);
            let spec = Arc::new(spec);
            runtime.block_on(exec_command_raw("pty_step", &spec, &test_exec_env()))
        };
        // Each mode's sleep has its own duration so a leftover process is
        // attributable to its test in a post-mortem.
        match mode {
            // One raw step: it echoes a start marker, backgrounds a long
            // sleep (recording its pid for the outer cleanup), and then
            // `exec`s a foreground sleep. The timeout is a backstop —
            // nothing under test relies on it.
            //
            // `exec sleep`, not `wait`: the step must *die of* the SIGINT
            // for the runner to relay it, and a shell interrupted in `wait`
            // may instead exit normally — macOS `/bin/sh` (bash 3.2) exits
            // 129, where dash dies of the signal. A plain `sleep` dies of
            // it everywhere.
            "ctrlc" | "stop" => {
                let sleep_secs = if mode == "ctrlc" { 301 } else { 302 };
                let script = format!(
                    "echo started; sleep {sleep_secs} & echo $! > \"{pid_file}\"; exec sleep {sleep_secs}"
                );
                let result = run(&script, 120);
                marker(&format!("STEP-DONE success={}\n", result.success));
            }
            // A step in the foreground that the terminal's hangup kills:
            // no `&`, so the sleep shares the step's group and dies with it.
            "hangup" => {
                let result = run("echo started; sleep 305", 120);
                marker(&format!("STEP-DONE success={}\n", result.success));
            }
            // `ops … &`: another group — standing in for the user's shell
            // running its own foreground job — holds the terminal, so the
            // raw step shares the inner's group. Its 1s timeout must still
            // kill it.
            "background" => {
                use std::os::unix::process::CommandExt as _;
                let mut holder = std::process::Command::new("sleep");
                holder
                    .arg("60")
                    .process_group(0)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null());
                let mut holder =
                    ops_core::subprocess::spawn_child_locked(&mut holder).expect("spawn holder");
                let holder_pid = libc::pid_t::try_from(holder.id()).expect("pid fits pid_t");
                // SAFETY: SIGTTOU is blocked on this thread (above), so the
                // inner may give the foreground away; tcsetpgrp only writes
                // the terminal's foreground-group attribute.
                unsafe {
                    libc::tcsetpgrp(0, holder_pid);
                }
                let result = run(&format!("echo $$$$ > \"{pid_file}\"; exec sleep 303"), 1);
                marker(&format!("STEP-DONE success={}\n", result.success));
                let _ = holder.kill();
                let _ = holder.wait();
            }
            // A timed-out step whose group ignores SIGTERM (so it outlives
            // its timeout by the whole teardown grace), then a step that
            // reads the terminal straight away: the second step must hold
            // the terminal, not stop on SIGTTIN behind the dying group.
            "timeout-next" => {
                let first = format!("trap '' TERM; echo $$$$ > \"{pid_file}\"; exec sleep 304");
                let result = run(&first, 1);
                marker(&format!("FIRST-DONE success={}\n", result.success));
                let result = run("read line; echo \"got:$$line\"", 30);
                marker(&format!("STEP-DONE success={}\n", result.success));
            }
            other => panic!("unknown inner mode {other}"),
        }
    }

    /// A pty pair: the master stays with the outer test, the slave path and
    /// a pid file are handed to the inner process.
    struct Pty {
        master: std::fs::File,
        slave_path: std::path::PathBuf,
    }

    fn open_pty() -> Pty {
        // ptsname returns a pointer into one static buffer, and the pty
        // tests run on parallel threads under the default test harness: the
        // call and the copy out of that buffer must not interleave with
        // another test's.
        static PTSNAME: std::sync::Mutex<()> = std::sync::Mutex::new(());
        // SAFETY: the libc pty calls only use the pointers/values owned by
        // this frame; ptsname's static buffer is read only while PTSNAME is
        // held, and copied out before it is released.
        unsafe {
            let master = libc::posix_openpt(libc::O_RDWR | libc::O_NOCTTY);
            assert!(master >= 0, "posix_openpt failed");
            assert_eq!(libc::grantpt(master), 0, "grantpt failed");
            assert_eq!(libc::unlockpt(master), 0, "unlockpt failed");
            let slave_path = {
                let _ptsname = PTSNAME.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                let name = libc::ptsname(master);
                assert!(!name.is_null(), "ptsname failed");
                std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned()
            };
            use std::os::unix::io::FromRawFd;
            Pty { master: std::fs::File::from_raw_fd(master), slave_path: slave_path.into() }
        }
    }

    /// Best-effort SIGKILL so a failing assertion cannot leak the inner
    /// process into the rest of the run.
    struct KillOnDrop(libc::pid_t);
    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            // SAFETY: kill(2) to a process we spawned; ESRCH is ignored.
            unsafe {
                libc::kill(self.0, libc::SIGKILL);
            }
        }
    }

    /// Best-effort SIGKILL of the process whose pid the inner step recorded,
    /// on every exit from the test, a failed assertion included. The
    /// recorded processes outlive the inner by design (a backgrounded sleep
    /// ignores the SIGINT that kills the inner; a SIGTERM-ignoring step
    /// outlives its own teardown), so the test must not leave them behind.
    struct KillRecordedPid(std::path::PathBuf);
    impl Drop for KillRecordedPid {
        fn drop(&mut self) {
            let Ok(text) = std::fs::read_to_string(&self.0) else { return };
            let Ok(pid) = text.trim().parse::<libc::pid_t>() else { return };
            // SAFETY: kill(2) to a process this test spawned; ESRCH means
            // it is already gone.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }
        }
    }

    /// Re-exec this test binary as the pty session leader in `mode`. The
    /// returned tempdir keeps the inner's pid file alive for the whole test
    /// (a dropped `TempDir` deletes it).
    #[allow(clippy::type_complexity)]
    fn spawn_inner(
        mode: &str,
    ) -> (Pty, KillOnDrop, libc::pid_t, std::path::PathBuf, tempfile::TempDir) {
        let pty = open_pty();
        let dir = tempfile::tempdir().expect("tempdir");
        let pid_file = dir.path().join("inner-sleep.pid");
        let mut inner = std::process::Command::new(std::env::current_exe().expect("current exe"));
        inner
            .args([
                "--exact",
                "--nocapture",
                "--test-threads=1",
                "command::tests::raw_mode::raw_foreground::pty_inner_raw_step",
            ])
            .env(INNER_MODE, mode)
            .env(INNER_PTS, &pty.slave_path)
            .env(INNER_PID_FILE, &pid_file)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit());
        let child = ops_core::subprocess::spawn_child_locked(&mut inner).expect("spawn inner");
        let pid = libc::pid_t::try_from(child.id()).expect("pid fits pid_t");
        // Dropping the std Child reaps nothing; the outer test reaps via
        // waitpid itself, with WUNTRACED to observe the Ctrl-Z stop.
        drop(child);
        (pty, KillOnDrop(pid), pid, pid_file, dir)
    }

    /// Read the master into `transcript` until `needle` appears. Bound by a
    /// 10s deadline; returns whether the needle was seen.
    ///
    /// A read can fail with `EIO` while no process holds the slave side
    /// open yet (between this test closing its own slave fd and the inner
    /// opening it) and after the inner dies; both are transients to poll
    /// through, not failures — only the deadline fails the wait.
    fn read_until(master: &std::fs::File, needle: &[u8], transcript: &mut Vec<u8>) -> bool {
        let started = std::time::Instant::now();
        while !subslice(transcript, needle) {
            if started.elapsed() >= Duration::from_secs(10) {
                return false;
            }
            let mut pfd = libc::pollfd { fd: master.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            // SAFETY: poll only reads/writes the pollfd owned by this frame.
            // SAFETY: poll only reads/writes the pollfd owned by this frame.
            if unsafe { libc::poll(&raw mut pfd, 1, 500) } <= 0 {
                continue;
            }
            let mut chunk = [0u8; 256];
            // SAFETY: read writes into `chunk`, owned by this frame.
            let n = unsafe {
                libc::read(
                    master.as_raw_fd(),
                    chunk.as_mut_ptr().cast::<libc::c_void>(),
                    chunk.len(),
                )
            };
            if n > 0 {
                transcript.extend_from_slice(&chunk[..usize::try_from(n).expect("read size")]);
            }
        }
        true
    }

    /// Drain whatever is immediately available into `transcript`.
    fn drain_available(master: &std::fs::File, transcript: &mut Vec<u8>) {
        let mut chunk = [0u8; 256];
        loop {
            let mut pfd = libc::pollfd { fd: master.as_raw_fd(), events: libc::POLLIN, revents: 0 };
            // SAFETY: as in read_until.
            if unsafe { libc::poll(&raw mut pfd, 1, 0) } <= 0 {
                return;
            }
            // SAFETY: as in read_until.
            let n = unsafe {
                libc::read(
                    master.as_raw_fd(),
                    chunk.as_mut_ptr().cast::<libc::c_void>(),
                    chunk.len(),
                )
            };
            if n <= 0 {
                return;
            }
            transcript.extend_from_slice(&chunk[..usize::try_from(n).expect("read size")]);
        }
    }

    fn subslice(haystack: &[u8], needle: &[u8]) -> bool {
        haystack.windows(needle.len()).any(|window| window == needle)
    }

    /// Wait until the pty's foreground group is no longer `pid`: the proof
    /// the raw step's foreground hand-off (or its return) happened.
    fn wait_until_foreground_is_not(master: &std::fs::File, pid: libc::pid_t) -> bool {
        let started = std::time::Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            // SAFETY: tcgetpgrp only inspects the descriptor's terminal.
            let fg = unsafe { libc::tcgetpgrp(master.as_raw_fd()) };
            if fg >= 0 && fg != pid {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    /// Wait until the inner process reports a stop (the Ctrl-Z dance). The
    /// stop event is consumed here: a WUNTRACED-reported stop is not
    /// re-reported to the later `reap_exited`, so this is the only place it
    /// can be observed.
    fn wait_until_stopped(pid: libc::pid_t) -> bool {
        let started = std::time::Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            let mut raw: libc::c_int = 0;
            // SAFETY: waitpid writes a complete status through `raw`; the
            // child is ours and nobody else waits on it.
            let reaped =
                unsafe { libc::waitpid(pid, &raw mut raw, libc::WNOHANG | libc::WUNTRACED) };
            if reaped == pid {
                if libc::WIFSTOPPED(raw) {
                    return true;
                }
                // Exited before stopping at all: let the caller's reaper
                // handle it.
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    /// Reap the inner process once it exits; counts intervening stops.
    ///
    /// Drains the master into `transcript` while it waits: on macOS a
    /// session leader's exit blocks until its tty's output queue (the `^C`
    /// echo, say) has been read, so an unread master would park the inner
    /// in exit and the reap would time out. Linux discards the queue.
    fn reap_exited(
        pid: libc::pid_t,
        master: &std::fs::File,
        transcript: &mut Vec<u8>,
        stops: &mut u32,
    ) -> Option<libc::c_int> {
        let started = std::time::Instant::now();
        while started.elapsed() < Duration::from_secs(10) {
            drain_available(master, transcript);
            let mut raw: libc::c_int = 0;
            // SAFETY: as in wait_until_stopped.
            let reaped =
                unsafe { libc::waitpid(pid, &raw mut raw, libc::WNOHANG | libc::WUNTRACED) };
            if reaped == pid {
                if libc::WIFSTOPPED(raw) {
                    *stops = stops.saturating_add(1);
                    continue;
                }
                return Some(raw);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        None
    }

    /// Send one byte to the pty master (VINTR / VSUSP with the default
    /// termios).
    fn send_key(master: &std::fs::File, byte: u8) {
        // SAFETY: write(2) only reads from the provided one-byte buffer.
        unsafe {
            libc::write(master.as_raw_fd(), (&raw const byte).cast::<libc::c_void>(), 1);
        }
    }

    /// A Ctrl-C at the terminal during a raw step must still cancel the run
    /// as it does without the hand-off. The inner process holds the pty's
    /// foreground and runs one raw step, which the hand-off gives the
    /// foreground to; the Ctrl-C therefore lands on the step's group only.
    /// The step dies of SIGINT and the runner-side relay must re-raise it
    /// on the inner process itself — which, with no handler installed,
    /// kills it exactly the way the shared-group design did. Asserts the
    /// relay death, the whole step group going with it, and that the step
    /// never reported completion.
    #[test]
    fn ctrl_c_on_the_pty_still_cancels_the_run() {
        let (pty, _kill, inner, pid_file, _dir) = spawn_inner("ctrlc");
        // POSIX makes an asynchronous (`&`) job ignore SIGINT, so the
        // step's backgrounded sleep survives the Ctrl-C exactly as it would
        // without the hand-off — the timeout teardown (whose SIGTERM /
        // SIGKILL cannot be ignored) is what owns that case, and it has its
        // own test above. Kill it here so it cannot leak into the run.
        let _sleep = KillRecordedPid(pid_file);
        let mut transcript = Vec::new();
        assert!(
            read_until(&pty.master, b"INNER-READY", &mut transcript),
            "inner never became ready; transcript: {transcript:?}"
        );
        assert!(
            wait_until_foreground_is_not(&pty.master, inner),
            "the raw step never received the foreground"
        );
        assert!(
            read_until(&pty.master, b"started", &mut transcript),
            "the step never started; transcript: {transcript:?}"
        );

        send_key(&pty.master, 3); // Ctrl-C

        let mut stops = 0;
        let status = reap_exited(inner, &pty.master, &mut transcript, &mut stops)
            .expect("inner must exit after the Ctrl-C");
        assert_eq!(stops, 0, "the runner must not stop on Ctrl-C");
        assert!(
            libc::WIFSIGNALED(status),
            "the runner must die of the relayed signal, got status {status}"
        );
        assert_eq!(
            libc::WTERMSIG(status),
            libc::SIGINT,
            "the relayed SIGINT must reach the runner process"
        );
        drain_available(&pty.master, &mut transcript);
        assert!(
            !subslice(&transcript, b"STEP-DONE"),
            "the step must not complete after cancelling the run: {transcript:?}"
        );
    }

    /// A Ctrl-Z at the terminal during a raw step must stop the step and
    /// the runner together — the job-control dance — and an `fg`-style
    /// resume (SIGCONT; the terminal is already the runner's again) must
    /// hand the foreground back to the step and continue it. The test then
    /// cancels with Ctrl-C and asserts the same relayed death as the
    /// Ctrl-C test.
    #[test]
    fn ctrl_z_stops_the_step_and_fg_resumes_it() {
        let (pty, _kill, inner, pid_file, _dir) = spawn_inner("stop");
        // The backgrounded sleep ignores the final Ctrl-C; see the Ctrl-C
        // test.
        let _sleep = KillRecordedPid(pid_file);
        let mut transcript = Vec::new();
        assert!(
            read_until(&pty.master, b"INNER-READY", &mut transcript),
            "inner never became ready; transcript: {transcript:?}"
        );
        assert!(
            wait_until_foreground_is_not(&pty.master, inner),
            "the raw step never received the foreground"
        );
        assert!(
            read_until(&pty.master, b"started", &mut transcript),
            "the step never started; transcript: {transcript:?}"
        );

        send_key(&pty.master, 0x1a); // Ctrl-Z

        // Exactly one stop must be observed, and it is observed here (the
        // reaper cannot see it again).
        let stopped = wait_until_stopped(inner);
        assert!(stopped, "the runner must stop itself with the Ctrl-Z'd step");
        // Resume like the shell's `fg` would: SIGCONT. The runner took the
        // terminal before stopping, so it still holds the foreground to
        // hand back to the step.
        //
        // SAFETY: kill(2) to our own child; the disposition is a plain
        // continue.
        unsafe {
            libc::kill(inner, libc::SIGCONT);
        }
        assert!(
            wait_until_foreground_is_not(&pty.master, inner),
            "the resumed step must get the foreground back"
        );

        send_key(&pty.master, 3); // Ctrl-C

        let mut extra_stops = 0;
        let status = reap_exited(inner, &pty.master, &mut transcript, &mut extra_stops)
            .expect("inner must exit after the Ctrl-C");
        assert_eq!(extra_stops, 0, "no stop may happen after the resume");
        assert!(
            libc::WIFSIGNALED(status),
            "the runner must die of the relayed signal, got status {status}"
        );
        assert_eq!(
            libc::WTERMSIG(status),
            libc::SIGINT,
            "the relayed SIGINT must reach the runner process"
        );
        drain_available(&pty.master, &mut transcript);
        assert!(
            !subslice(&transcript, b"STEP-DONE"),
            "the step must not complete after cancelling the run: {transcript:?}"
        );
    }

    /// When the runner is a background job (`ops … &`), the raw step stays
    /// in the runner's group, where `killpg` on the child's pid reaches
    /// nothing. Its timeout must still kill it — the legacy direct-child
    /// kill — rather than leave it running behind a returned step.
    #[test]
    fn raw_timeout_in_a_background_run_kills_the_step() {
        let (pty, _kill, _inner, pid_file, _dir) = spawn_inner("background");
        let _step = KillRecordedPid(pid_file.clone());
        let mut transcript = Vec::new();
        assert!(
            read_until(&pty.master, b"STEP-DONE", &mut transcript),
            "the timed-out step never returned; transcript: {transcript:?}"
        );
        assert!(
            subslice(&transcript, b"STEP-DONE success=false"),
            "a timed-out raw step must fail: {transcript:?}"
        );
        let recorded = std::fs::read_to_string(&pid_file).unwrap_or_default();
        let pid: libc::pid_t = recorded.trim().parse().unwrap_or_else(|_| {
            panic!("step pid file held {recorded:?}; transcript: {transcript:?}")
        });
        assert!(
            wait_until_dead(pid, Duration::from_secs(10)),
            "the background run's raw step {pid} survived its timeout"
        );
    }

    /// A timed-out step whose group ignores SIGTERM keeps running through
    /// the whole teardown grace. The next raw step starts immediately and
    /// reads the terminal: the timeout must already have taken the terminal
    /// back, so the next step is handed it rather than stopping on SIGTTIN
    /// behind the dying group.
    #[test]
    fn timed_out_step_returns_the_terminal_before_the_next_step() {
        let (pty, _kill, _inner, pid_file, _dir) = spawn_inner("timeout-next");
        let _first = KillRecordedPid(pid_file);
        let mut transcript = Vec::new();
        assert!(
            read_until(&pty.master, b"FIRST-DONE", &mut transcript),
            "the first step never timed out; transcript: {transcript:?}"
        );
        {
            use std::io::Write as _;
            (&pty.master).write_all(b"hello\n").expect("write to the pty");
        }
        assert!(
            read_until(&pty.master, b"got:hello", &mut transcript),
            "the next step never read the terminal; transcript: {transcript:?}"
        );
        assert!(
            read_until(&pty.master, b"STEP-DONE success=true", &mut transcript),
            "the next step must succeed; transcript: {transcript:?}"
        );
    }

    /// Closing the terminal must still end the run. A hangup delivers
    /// `SIGHUP` to the terminal's foreground group, which during a raw step
    /// is the step's group only; the step dies of it and the runner-side
    /// relay must re-raise it on the inner process, which — with no handler
    /// installed — dies of it exactly as the shared-group design did. The
    /// test sends the hangup's `SIGHUP` to the foreground group the way the
    /// terminal does, rather than closing the master, because a real hangup
    /// also signals the session leader (the inner itself) directly and
    /// could not tell the relay apart from that.
    #[test]
    fn terminal_hangup_on_the_pty_still_ends_the_run() {
        let (pty, _kill, inner, _pid_file, _dir) = spawn_inner("hangup");
        let mut transcript = Vec::new();
        assert!(
            read_until(&pty.master, b"INNER-READY", &mut transcript),
            "inner never became ready; transcript: {transcript:?}"
        );
        assert!(
            wait_until_foreground_is_not(&pty.master, inner),
            "the raw step never received the foreground"
        );
        assert!(
            read_until(&pty.master, b"started", &mut transcript),
            "the step never started; transcript: {transcript:?}"
        );
        // SAFETY: tcgetpgrp only inspects the master's terminal.
        let step_group = unsafe { libc::tcgetpgrp(pty.master.as_raw_fd()) };
        assert!(step_group > 0 && step_group != inner, "the step must hold the foreground");
        // SAFETY: killpg to the step group the inner spawned; ESRCH if it
        // is already gone would fail the assertions below, not memory.
        unsafe {
            libc::killpg(step_group, libc::SIGHUP);
        }

        let mut stops = 0;
        let status = reap_exited(inner, &pty.master, &mut transcript, &mut stops)
            .expect("inner must exit after the hangup");
        assert_eq!(stops, 0, "the runner must not stop on a hangup");
        assert!(
            libc::WIFSIGNALED(status),
            "the runner must die of the relayed signal, got status {status}"
        );
        assert_eq!(
            libc::WTERMSIG(status),
            libc::SIGHUP,
            "the relayed SIGHUP must reach the runner process"
        );
        drain_available(&pty.master, &mut transcript);
        assert!(
            !subslice(&transcript, b"STEP-DONE"),
            "the run must not continue after the terminal closed: {transcript:?}"
        );
    }
}
