//! End-to-end tests for `ops lock` (TASK-2281): the lock is shared by every
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
        .env_remove("GIT_INDEX_FILE");
    cmd
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .args(args)
        .status()
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
    git(
        &repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "side",
            wt.to_str().expect("utf8"),
        ],
    );
    (dir, repo, wt)
}

fn status(dir: &Path, name: &str) -> String {
    let out = ops(dir)
        .args(["lock", "status", name])
        .output()
        .expect("status");
    assert!(out.status.success(), "status failed: {out:?}");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
    let deadline = Instant::now()
        .checked_add(Duration::from_secs(20))
        .expect("deadline");
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
    let mut holder = ops(&repo)
        .args(["lock", "merge", "--", "sleep", "30"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn holder");
    wait_until("the holder to take the lock", || {
        status(&wt, "merge").contains("held (alive)")
    });
    let seen = status(&wt, "merge");
    assert!(seen.contains("sleep 30"), "command recorded: {seen}");
    assert!(seen.contains(repo.file_name().and_then(|n| n.to_str()).expect("name")));

    let contended = ops(&wt)
        .args(["lock", "merge", "--timeout", "0", "--", "true"])
        .output()
        .expect("contender");
    assert!(!contended.status.success(), "must not acquire a held lock");

    let pid = libc::pid_t::try_from(holder.id()).expect("pid");
    // SAFETY: `kill(2)` on our own child with a plain signal number.
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
    let exit = holder.wait().expect("holder exit");
    assert_eq!(exit.code(), Some(143), "128 + SIGTERM");
    assert!(status(&wt, "merge").contains("merge: free"));
}

/// AC #1 (non-zero exit): the command's failure is the exit code, and the
/// lock is free afterwards.
#[test]
fn a_failing_command_releases_the_lock_and_propagates_its_code() {
    let (_dir, repo, _wt) = repo_with_worktree();
    let out = ops(&repo)
        .args(["lock", "merge", "--", "sh", "-c", "exit 7"])
        .output()
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
    let mut holder = ops(&repo)
        .args(["lock", "merge", "--", "sleep", "30"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn holder");
    wait_until("the holder to take the lock", || {
        status(&repo, "merge").contains("held (alive)")
    });
    holder.kill().expect("SIGKILL");
    holder.wait().expect("reap");
    // The orphaned `sleep` does not inherit the lock (the fd is CLOEXEC).
    let seen = status(&repo, "merge");
    assert!(seen.contains("merge: stale"), "got: {seen}");
    let out = ops(&repo)
        .args(["lock", "merge", "--timeout", "0", "--", "true"])
        .output()
        .expect("reacquire");
    assert!(out.status.success(), "stale record must not block: {out:?}");
}
