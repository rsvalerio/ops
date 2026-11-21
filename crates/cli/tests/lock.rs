//! End-to-end tests for `ops lock`: the lock is shared by every
//! worktree of a repository, and it is released when the holder is
//! signalled or its command fails.

// An integration-test target is its own crate (docs/clippy.md layer 2): the
// helpers below are fixtures where a panic is the desired failure mode.
#![allow(clippy::expect_used)]
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn ops_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ops"))
}

fn ops(dir: &Path) -> Command {
    let mut cmd = Command::new(ops_bin());
    cmd.current_dir(dir)
        .env("HOME", dir)
        .env("XDG_CONFIG_HOME", dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("OPS_LOCKS_HELD");
    cmd
}

fn git(dir: &Path, args: &[&str]) {
    let status = ops_core::subprocess::status_locked(
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .args(args),
    )
    .expect("spawn git");
    assert!(status.success(), "git {args:?}");
}

/// `<tmp>/repo` with one commit and a linked worktree at `<tmp>/wt`.
fn repo_with_worktree() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).expect("repo dir");
    git(&repo, &["init", "-q", "-b", "main"]);
    git(
        &repo,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.com",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "seed",
        ],
    );
    let wt = dir.path().join("wt");
    git(&repo, &["worktree", "add", "-q", "-b", "side", wt.to_str().expect("utf8")]);
    (dir, repo, wt)
}

fn status(dir: &Path, name: &str) -> String {
    let out = ops_core::subprocess::output_locked(ops(dir).args(["lock", "status", name]))
        .expect("status");
    assert!(out.status.success(), "status failed: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now().checked_add(Duration::from_secs(20)).expect("deadline");
    while !cond() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// AC #3 and AC #1 (SIGTERM): a lock taken in the main checkout is seen as
/// held from the linked worktree, blocks an acquirer there, and is released
/// — record cleared — when the holder is sent SIGTERM.
#[test]
fn lock_is_shared_across_worktrees_and_released_on_sigterm() {
    let (_dir, repo, wt) = repo_with_worktree();
    let mut holder = ops_core::subprocess::spawn_child_locked(
        ops(&repo)
            .args(["lock", "merge", "--", "sleep", "30"])
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )
    .expect("spawn holder");
    wait_until("the holder to take the lock", || status(&wt, "merge").contains("held (alive)"));
    let seen = status(&wt, "merge");
    assert!(seen.contains("sleep 30"), "command recorded: {seen}");
    assert!(seen.contains(repo.file_name().and_then(|n| n.to_str()).expect("name")));

    let contended = ops_core::subprocess::output_locked(ops(&wt).args([
        "lock",
        "merge",
        "--timeout",
        "0",
        "--",
        "true",
    ]))
    .expect("contender");
    assert!(!contended.status.success(), "must not acquire a held lock");

    signal(&holder, libc::SIGTERM);
    let exit = holder.wait().expect("holder exit");
    assert_eq!(exit.code(), Some(143), "128 + SIGTERM");
    assert!(status(&wt, "merge").contains("merge: free"));
}

/// AC #1 (non-zero exit): the command's failure is the exit code, and the
/// lock is free afterwards.
#[test]
fn a_failing_command_releases_the_lock_and_propagates_its_code() {
    let (_dir, repo, _wt) = repo_with_worktree();
    let out = ops_core::subprocess::output_locked(
        ops(&repo).args(["lock", "merge", "--", "sh", "-c", "exit 7"]),
    )
    .expect("run");
    assert_eq!(out.status.code(), Some(7));
    assert!(status(&repo, "merge").contains("merge: free"));
}

/// A holder killed outright (SIGKILL: no cleanup runs) leaves its record;
/// the OS released the lock, so status reports it stale and a new acquirer
/// is not blocked.
#[test]
fn a_killed_holder_is_reported_stale_and_does_not_block() {
    let (_dir, repo, _wt) = repo_with_worktree();
    let mut holder = ops_core::subprocess::spawn_child_locked(
        ops(&repo)
            .args(["lock", "merge", "--", "sleep", "30"])
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )
    .expect("spawn holder");
    wait_until("the holder to take the lock", || status(&repo, "merge").contains("held (alive)"));
    holder.kill().expect("SIGKILL");
    holder.wait().expect("reap");
    // The orphaned `sleep` does not inherit the lock (the fd is CLOEXEC).
    let seen = status(&repo, "merge");
    assert!(seen.contains("merge: stale"), "got: {seen}");
    let out = ops_core::subprocess::output_locked(ops(&repo).args([
        "lock",
        "merge",
        "--timeout",
        "0",
        "--",
        "true",
    ]))
    .expect("reacquire");
    assert!(out.status.success(), "stale record must not block: {out:?}");
}

/// Send `signo` to `child`.
#[expect(unsafe_code, reason = "libc::kill FFI to signal the child under test")]
fn signal(child: &std::process::Child, signo: libc::c_int) {
    let pid = libc::pid_t::try_from(child.id()).expect("pid");
    // SAFETY: `kill(2)` on our own not-yet-reaped child with a plain signal
    // number.
    unsafe {
        libc::kill(pid, signo);
    }
}

/// a SIGTERM sent the moment the holder's record appears —
/// before `ops` has started the command — still exits `128 + SIGTERM` and
/// leaves the lock free, not stale. The record file is polled directly
/// (spawning `ops lock status` is too slow to land in that window), and the
/// run is repeated to hit it reliably.
#[test]
fn sigterm_right_after_the_lock_is_taken_still_releases_it() {
    let (_dir, repo, _wt) = repo_with_worktree();
    let record = repo.join(".git/ops/locks/merge.lock");
    for _ in 0..20 {
        let mut holder = ops_core::subprocess::spawn_child_locked(
            ops(&repo)
                .args(["lock", "merge", "--", "sleep", "30"])
                .stdout(Stdio::null())
                .stderr(Stdio::null()),
        )
        .expect("spawn holder");
        let deadline = Instant::now().checked_add(Duration::from_secs(20)).expect("deadline");
        while !std::fs::metadata(&record).is_ok_and(|m| m.len() > 0) {
            assert!(Instant::now() < deadline, "the holder never took the lock");
        }
        signal(&holder, libc::SIGTERM);
        let exit = holder.wait().expect("holder exit");
        assert_eq!(exit.code(), Some(143), "128 + SIGTERM");
        let seen = status(&repo, "merge");
        assert!(seen.contains("merge: free"), "got: {seen}");
    }
}

/// a signal while waiting for a held lock ends the wait with
/// `128 + signo`; the command never runs and the holder keeps the lock.
#[test]
fn a_signal_while_waiting_ends_the_wait_without_running_the_command() {
    use std::io::BufRead as _;

    let (dir, repo, wt) = repo_with_worktree();
    let mut holder = ops_core::subprocess::spawn_child_locked(
        ops(&repo)
            .args(["lock", "merge", "--", "sleep", "30"])
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )
    .expect("spawn holder");
    wait_until("the holder to take the lock", || status(&wt, "merge").contains("held (alive)"));

    let marker = dir.path().join("ran");
    let mut waiter = ops_core::subprocess::spawn_child_locked(
        ops(&wt)
            .args(["lock", "merge", "--", "touch"])
            .arg(&marker)
            .stdout(Stdio::null())
            .stderr(Stdio::piped()),
    )
    .expect("spawn waiter");
    let mut notice = String::new();
    std::io::BufReader::new(waiter.stderr.take().expect("stderr"))
        .read_line(&mut notice)
        .expect("read notice");
    assert!(notice.contains("waiting for merge"), "got: {notice}");

    signal(&waiter, libc::SIGINT);
    let exit = waiter.wait().expect("waiter exit");
    assert_eq!(exit.code(), Some(130), "128 + SIGINT");
    assert!(!marker.exists(), "the command must not run");
    assert!(status(&wt, "merge").contains("sleep 30"), "holder keeps it");

    signal(&holder, libc::SIGTERM);
    holder.wait().expect("holder exit");
}

/// A pseudo-terminal pair: `(master, slave)`.
#[expect(unsafe_code, reason = "openpty FFI to give the holder a real terminal")]
fn openpty() -> (std::os::fd::OwnedFd, std::os::fd::OwnedFd) {
    use std::os::fd::FromRawFd as _;
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: `openpty` writes two descriptors into the out-pointers, which
    // point at live locals; the name/termios/winsize arguments are null,
    // which it documents as "use the defaults".
    let rc = unsafe {
        libc::openpty(
            &raw mut master,
            &raw mut slave,
            // `null_mut` for `termp`/`winp`: macOS declares them `*mut`,
            // Linux `*const`, and `*mut` coerces to `*const`.
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, 0, "openpty: {}", std::io::Error::last_os_error());
    // SAFETY: `openpty` succeeded, so both are fresh descriptors we own.
    unsafe { (std::os::fd::OwnedFd::from_raw_fd(master), std::os::fd::OwnedFd::from_raw_fd(slave)) }
}

/// Run `cmd` as the session leader of the terminal behind its stdin, the
/// way a shell runs a foreground job: its process group is the terminal's
/// foreground group, so a Ctrl-C written to the master reaches the group.
#[expect(unsafe_code, reason = "setsid/ioctl in pre_exec to take the pty as controlling tty")]
fn as_terminal_session(cmd: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    // macOS types `TIOCSCTTY` as `c_uint` while `ioctl` takes a `c_ulong`
    // request; on Linux the two already match.
    #[cfg(target_vendor = "apple")]
    let request = libc::c_ulong::from(libc::TIOCSCTTY);
    #[cfg(not(target_vendor = "apple"))]
    let request = libc::TIOCSCTTY;
    // SAFETY: the closure runs in the forked child before exec and calls
    // only async-signal-safe functions (`setsid`, `ioctl`) on plain
    // integers; stdin is already the pty slave by then.
    unsafe {
        cmd.pre_exec(move || {
            if libc::setsid() < 0 || libc::ioctl(0, request, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

/// Reads a trap's counter file. The trap's `echo > file` truncates before it
/// writes, so a poll can observe the file empty mid-update; that reads as "not
/// counted yet" rather than a parse failure.
fn read_count(path: &Path) -> u32 {
    std::fs::read_to_string(path).map_or(0, |s| {
        let s = s.trim();
        if s.is_empty() {
            0
        } else {
            s.parse().expect("count")
        }
    })
}

/// A single Ctrl-C at the terminal reaches the locked command exactly once:
/// the tty already delivers it to the whole foreground group, so `ops` must
/// not forward a second copy. A SIGTERM sent to `ops` alone is still
/// forwarded, once.
#[test]
fn one_ctrl_c_at_the_terminal_reaches_the_locked_command_once() {
    use std::io::Write as _;

    let (dir, repo, _wt) = repo_with_worktree();
    let [ints, terms, ready, stop] =
        ["ints", "terms", "ready", "stop"].map(|name| dir.path().join(name));
    // Counts each SIGINT / SIGTERM it receives into a file, then spins until
    // told to stop. The loop is builtins only: a shell runs a trap only
    // between commands, so waiting on an external `sleep` would let the tty
    // copy and a forwarded duplicate coalesce into one pending trap and hide
    // the bug this test exists to catch.
    let script = r#"n=0; t=0
trap 'n=$((n+1)); echo $n > "$1"' INT
trap 't=$((t+1)); echo $t > "$2"' TERM
: > "$3"
while [ ! -e "$4" ]; do :; done"#;

    let (master, slave) = openpty();
    let mut cmd = ops(&repo);
    cmd.args(["lock", "merge", "--", "sh", "-c", script, "sh"])
        .args([&ints, &terms, &ready, &stop])
        .stdin(Stdio::from(slave.try_clone().expect("dup slave")))
        .stdout(Stdio::from(slave.try_clone().expect("dup slave")))
        .stderr(Stdio::from(slave));
    as_terminal_session(&mut cmd);
    let mut holder = ops_core::subprocess::spawn_child_locked(&mut cmd).expect("spawn holder");
    drop(cmd);
    wait_until("the locked command to start", || ready.exists());

    let mut tty = std::fs::File::from(master);
    tty.write_all(&[0x03]).expect("type Ctrl-C");
    wait_until("the SIGINT to arrive", || read_count(&ints) > 0);
    // Leave room for a forwarded duplicate to land before counting.
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(read_count(&ints), 1, "one Ctrl-C must deliver exactly one SIGINT");

    signal(&holder, libc::SIGTERM);
    wait_until("the forwarded SIGTERM to arrive", || read_count(&terms) > 0);
    std::fs::write(&stop, "").expect("stop");
    holder.wait().expect("holder exit");
    assert_eq!(read_count(&terms), 1, "SIGTERM to ops is forwarded once");
    assert_eq!(read_count(&ints), 1, "no SIGINT arrives late");
}

/// `ops backlog commit` waits while a wave holds `code-review-merge`, so a
/// bookkeeping commit cannot land between a wave's integration verify and
/// its fast-forward merge.
#[test]
fn backlog_commit_waits_for_the_merge_lock() {
    let (_dir, repo, wt) = repo_with_worktree();
    let mut holder = ops_core::subprocess::spawn_child_locked(
        ops(&repo)
            .args(["lock", "code-review-merge", "--", "sleep", "30"])
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )
    .expect("spawn holder");
    wait_until("the holder to take the lock", || {
        status(&wt, "code-review-merge").contains("held (alive)")
    });

    let mut commit = ops_core::subprocess::spawn_child_locked(
        ops(&wt)
            .args(["backlog", "commit", "TASK-1", "-m", "chore(backlog): x"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped()),
    )
    .expect("spawn commit");
    std::thread::sleep(Duration::from_millis(500));
    assert!(commit.try_wait().expect("poll").is_none(), "commit must wait for the merge lock");

    signal(&holder, libc::SIGTERM);
    holder.wait().expect("holder exit");
    let out = commit.wait_with_output().expect("commit exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("waiting for code-review-merge"), "got: {stderr}");
}

/// Under an enclosing `ops lock code-review-merge`, `ops backlog commit`
/// does not wait on its own parent.
#[test]
fn backlog_commit_inside_the_merge_lock_does_not_deadlock() {
    let (_dir, repo, _wt) = repo_with_worktree();
    let mut nested = ops_core::subprocess::spawn_child_locked(
        ops(&repo)
            .args(["lock", "code-review-merge", "--", ops_bin().to_str().expect("utf8")])
            .args(["backlog", "commit", "TASK-1", "-m", "chore(backlog): x"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped()),
    )
    .expect("spawn nested");
    wait_until("the nested commit to finish", || nested.try_wait().expect("poll").is_some());
    let out = nested.wait_with_output().expect("nested exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("waiting for"), "got: {stderr}");
}

/// The held lock is matched by its lock file, not by how the path to it is
/// spelled: a lock taken from the main checkout (where git reports the
/// common dir as a relative `.git`) still covers a nested commit run from a
/// linked worktree (where it is absolute).
#[test]
fn backlog_commit_in_a_linked_worktree_inside_the_merge_lock_does_not_deadlock() {
    let (_dir, repo, wt) = repo_with_worktree();
    let script = format!(
        "cd \"$1\" && exec {} backlog commit TASK-1 -m 'chore(backlog): x'",
        ops_bin().to_str().expect("utf8")
    );
    let mut nested = ops_core::subprocess::spawn_child_locked(
        ops(&repo)
            .args(["lock", "code-review-merge", "--", "sh", "-c", &script, "sh"])
            .arg(&wt)
            .stdout(Stdio::null())
            .stderr(Stdio::piped()),
    )
    .expect("spawn nested");
    wait_until("the nested commit to finish", || nested.try_wait().expect("poll").is_some());
    let out = nested.wait_with_output().expect("nested exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("waiting for"), "got: {stderr}");
}

/// Holding `code-review-merge` in one repository does not exempt a commit
/// in another: that repository's own merge lock is still taken, so the
/// commit waits for its holder.
#[test]
fn a_merge_lock_held_in_one_repository_does_not_cover_another() {
    let (_dir_a, repo_a, _wt_a) = repo_with_worktree();
    let (_dir_b, repo_b, wt_b) = repo_with_worktree();
    let mut holder_b = ops_core::subprocess::spawn_child_locked(
        ops(&repo_b)
            .args(["lock", "code-review-merge", "--", "sleep", "30"])
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )
    .expect("spawn holder");
    wait_until("the holder to take repository B's lock", || {
        status(&wt_b, "code-review-merge").contains("held (alive)")
    });

    let script = format!(
        "cd \"$1\" && exec {} backlog commit TASK-1 -m 'chore(backlog): x'",
        ops_bin().to_str().expect("utf8")
    );
    let mut nested = ops_core::subprocess::spawn_child_locked(
        ops(&repo_a)
            .args(["lock", "code-review-merge", "--", "sh", "-c", &script, "sh"])
            .arg(&repo_b)
            .stdout(Stdio::null())
            .stderr(Stdio::piped()),
    )
    .expect("spawn nested");
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        nested.try_wait().expect("poll").is_none(),
        "the commit in repository B must wait for B's merge lock"
    );

    signal(&holder_b, libc::SIGTERM);
    holder_b.wait().expect("holder exit");
    let out = nested.wait_with_output().expect("nested exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("waiting for code-review-merge"), "got: {stderr}");
}

/// A job started under `ops lock` that outlives it (a background job) keeps
/// the holder's `OPS_LOCKS_HELD`. Once that holder has exited and another
/// process holds the lock, the inherited entry no longer covers the job:
/// its `ops backlog commit` waits like any other.
#[test]
fn an_inherited_lock_does_not_cover_a_job_that_outlives_its_holder() {
    let (_dir, repo, wt) = repo_with_worktree();
    let out = ops_core::subprocess::output_locked(ops(&repo).args([
        "lock",
        "code-review-merge",
        "--",
        "printenv",
        "OPS_LOCKS_HELD",
    ]))
    .expect("run first holder");
    assert!(out.status.success(), "first holder: {out:?}");
    let leaked = String::from_utf8(out.stdout).expect("utf8").trim_end().to_string();
    assert!(!leaked.is_empty(), "the locked command sees OPS_LOCKS_HELD");

    let mut holder = ops_core::subprocess::spawn_child_locked(
        ops(&repo)
            .args(["lock", "code-review-merge", "--", "sleep", "30"])
            .stdout(Stdio::null())
            .stderr(Stdio::null()),
    )
    .expect("spawn second holder");
    wait_until("the second holder to take the lock", || {
        status(&wt, "code-review-merge").contains("held (alive)")
    });

    let mut commit = ops_core::subprocess::spawn_child_locked(
        ops(&wt)
            .env("OPS_LOCKS_HELD", &leaked)
            .args(["backlog", "commit", "TASK-1", "-m", "chore(backlog): x"])
            .stdout(Stdio::null())
            .stderr(Stdio::piped()),
    )
    .expect("spawn commit");
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        commit.try_wait().expect("poll").is_none(),
        "a stale inherited entry must not skip the merge lock"
    );

    signal(&holder, libc::SIGTERM);
    holder.wait().expect("holder exit");
    let out = commit.wait_with_output().expect("commit exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("waiting for code-review-merge"), "got: {stderr}");
}
