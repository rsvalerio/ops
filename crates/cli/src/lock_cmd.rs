//! `ops lock`: named advisory locks shared by every worktree of a
//! repository.
//!
//! `ops lock <name> -- <cmd…>` runs a command while holding the lock;
//! `ops lock status [<name>]` reports holders; `ops lock break <name>`
//! clears a dead holder's record.
//!
//! # Why `flock` and not `mkdir`
//!
//! The code-review wave runner serialized merges with `mkdir <lock-dir>` and
//! a `trap 'rmdir …' EXIT`. A killed runner leaves the directory behind, and
//! nothing in it says who held it. Here the lock is an OS advisory lock
//! (`flock(2)` via [`std::fs::File::lock`]) on
//! `<git-common-dir>/ops/locks/<name>.lock`: the kernel drops it when the
//! holding process exits *by any means*, SIGKILL included, so a lock can
//! never outlive its holder. The file's contents are only the holder's
//! record — pid, host, worktree, command, acquisition time — written after
//! the lock is taken and cleared on a clean release. A record left behind
//! with the lock free is exactly "the holder died": that is what `status`
//! reports as stale. The file itself is never unlinked, which is what keeps
//! every waiter locking the same inode.
//!
//! The common git dir (not `.git` of one worktree) is what makes a lock
//! shared across all worktrees of one repository.
//!
//! # Signals
//!
//! While the command runs, SIGINT and SIGTERM delivered to `ops` are
//! forwarded to the child and `ops` keeps waiting for it, so the lock is held
//! for exactly the child's lifetime and released — record cleared — once it
//! exits. `ops` then exits `128 + signo`. If `ops` itself is killed
//! outright, the kernel releases the lock and the record reads as stale.

use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{Read as _, Seek as _, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Context as _;

/// How often a lock wait with a timeout re-polls.
const POLL: Duration = Duration::from_millis(100);

/// The lock directory for the repository containing `cwd`:
/// `<git-common-dir>/ops/locks`, created on demand.
///
/// # Errors
///
/// `cwd` is not inside a git repository, or the directory cannot be
/// created.
pub fn locks_dir(cwd: &Path) -> anyhow::Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .args(["rev-parse", "--git-common-dir"])
        .output()
        .context("running git rev-parse --git-common-dir")?;
    if !output.status.success() {
        anyhow::bail!(
            "ops lock needs a git repository: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let common = PathBuf::from(raw.trim_end_matches(['\n', '\r']));
    let common = if common.is_absolute() {
        common
    } else {
        cwd.join(common)
    };
    let dir = common.join("ops").join("locks");
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    Ok(dir)
}

/// A lock name becomes a file name, so it is restricted to a safe alphabet:
/// no separators, no `..`, no leading dot or dash.
///
/// # Errors
///
/// The name is empty, starts with `.` or `-`, or holds a character outside
/// `[A-Za-z0-9._-]`.
pub fn validate_name(name: &str) -> anyhow::Result<()> {
    let valid = !name.is_empty()
        && !name.starts_with(['.', '-'])
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if valid {
        Ok(())
    } else {
        anyhow::bail!(
            "invalid lock name {name:?}: use letters, digits, '.', '_' and '-', \
             not starting with '.' or '-'"
        )
    }
}

fn lock_path(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.lock"))
}

/// Who holds (or last held) a lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holder {
    pub pid: u32,
    pub host: String,
    pub worktree: String,
    pub command: String,
    /// Seconds since the Unix epoch when the lock was taken.
    pub acquired: u64,
}

impl Holder {
    fn current(worktree: &Path, command: &[OsString]) -> Self {
        Self {
            pid: std::process::id(),
            host: hostname(),
            worktree: worktree.display().to_string(),
            command: command
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(" "),
            acquired: now_secs(),
        }
    }

    /// `key=value` lines; newlines inside a value are flattened so a
    /// command cannot forge extra keys.
    fn render(&self) -> String {
        let clean = |v: &str| v.replace(['\n', '\r'], " ");
        format!(
            "pid={}\nhost={}\nworktree={}\ncommand={}\nacquired={}\n",
            self.pid,
            clean(&self.host),
            clean(&self.worktree),
            clean(&self.command),
            self.acquired
        )
    }

    /// Parse a record; `None` for an empty (released) or unreadable one.
    fn parse(text: &str) -> Option<Self> {
        let mut holder = Self {
            pid: 0,
            host: String::new(),
            worktree: String::new(),
            command: String::new(),
            acquired: 0,
        };
        let mut saw_pid = false;
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "pid" => {
                    holder.pid = value.parse().ok()?;
                    saw_pid = true;
                }
                "host" => holder.host = value.to_string(),
                "worktree" => holder.worktree = value.to_string(),
                "command" => holder.command = value.to_string(),
                "acquired" => holder.acquired = value.parse().unwrap_or(0),
                _ => {}
            }
        }
        saw_pid.then_some(holder)
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn hostname() -> String {
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// `12s`, `3m04s`, `2h05m`.
fn format_age(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m{:02}s", secs / 60, secs % 60)
    } else {
        format!("{}h{:02}m", secs / 3600, (secs % 3600) / 60)
    }
}

fn open_lock_file(path: &Path) -> anyhow::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))
}

fn read_record(file: &mut File) -> Option<Holder> {
    let mut text = String::new();
    file.rewind().ok()?;
    file.read_to_string(&mut text).ok()?;
    Holder::parse(&text)
}

fn write_record(file: &mut File, record: &str) -> std::io::Result<()> {
    file.set_len(0)?;
    file.rewind()?;
    file.write_all(record.as_bytes())?;
    file.sync_data()
}

/// A held lock. Dropping it clears the record and releases the lock.
#[derive(Debug)]
pub struct Guard {
    file: File,
}

impl Drop for Guard {
    fn drop(&mut self) {
        // Clear the record first so a reader never sees a live-looking
        // record on a free lock; the lock itself goes with the handle.
        write_record(&mut self.file, "").ok();
        self.file.unlock().ok();
    }
}

/// Take the lock `name` in `dir`, waiting while another process holds it.
/// `timeout: None` waits indefinitely. Writes a one-line notice to `notice`
/// when it has to wait.
///
/// # Errors
///
/// The name is invalid, the lock file cannot be opened or locked, the
/// timeout elapsed (the error names the holder), or the record cannot be
/// written.
pub fn acquire(
    dir: &Path,
    name: &str,
    holder: &Holder,
    timeout: Option<Duration>,
    notice: &mut dyn Write,
) -> anyhow::Result<Guard> {
    validate_name(name)?;
    let path = lock_path(dir, name);
    let mut file = open_lock_file(&path)?;
    match file.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => {
            let who = read_record(&mut file).map_or_else(
                || "another process".to_string(),
                |h| format!("pid {} ({})", h.pid, h.command),
            );
            writeln!(notice, "ops lock: waiting for {name}, held by {who}").ok();
            match timeout {
                None => file
                    .lock()
                    .with_context(|| format!("locking {}", path.display()))?,
                Some(limit) => wait_with_timeout(&file, &path, name, &who, limit)?,
            }
        }
        Err(std::fs::TryLockError::Error(err)) => {
            return Err(err).with_context(|| format!("locking {}", path.display()));
        }
    }
    write_record(&mut file, &holder.render())
        .with_context(|| format!("recording the holder in {}", path.display()))?;
    Ok(Guard { file })
}

fn wait_with_timeout(
    file: &File,
    path: &Path,
    name: &str,
    who: &str,
    limit: Duration,
) -> anyhow::Result<()> {
    let deadline = Instant::now().checked_add(limit);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(std::fs::TryLockError::Error(err)) => {
                return Err(err).with_context(|| format!("locking {}", path.display()));
            }
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            anyhow::bail!(
                "timed out after {}s waiting for lock {name}, held by {who}",
                limit.as_secs()
            );
        }
        std::thread::sleep(POLL);
    }
}

/// A lock's state as `status` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockState {
    /// No holder and no leftover record.
    Free,
    /// Held by a live process (the kernel lock is taken). The record may be
    /// `None` for the instant between locking and writing it.
    Held(Option<Holder>),
    /// The lock is free but a holder's record remains: that holder exited
    /// without releasing — killed, most likely.
    Stale(Holder),
}

/// Probe one lock without waiting.
///
/// # Errors
///
/// The lock file exists but cannot be opened or probed.
pub fn probe(dir: &Path, name: &str) -> anyhow::Result<LockState> {
    validate_name(name)?;
    let path = lock_path(dir, name);
    if !path.exists() {
        return Ok(LockState::Free);
    }
    let mut file = open_lock_file(&path)?;
    let record = read_record(&mut file);
    // A shared probe conflicts with a holder's exclusive lock but not with
    // another status probe.
    match file.try_lock_shared() {
        Ok(()) => {
            // Re-read under the lock: a holder may have released between
            // the first read and the probe.
            let record = read_record(&mut file);
            file.unlock().ok();
            Ok(record.map_or(LockState::Free, LockState::Stale))
        }
        Err(std::fs::TryLockError::WouldBlock) => Ok(LockState::Held(record)),
        Err(std::fs::TryLockError::Error(err)) => {
            Err(err).with_context(|| format!("probing {}", path.display()))
        }
    }
}

/// Every lock name with a file in `dir`, sorted.
///
/// # Errors
///
/// `dir` cannot be read.
fn lock_names(dir: &Path) -> anyhow::Result<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".lock").map(str::to_string)
        })
        .filter(|name| validate_name(name).is_ok())
        .collect();
    names.sort();
    Ok(names)
}

fn describe(holder: &Holder) -> String {
    format!(
        "pid {} on {}, worktree {}, age {}, command: {}",
        holder.pid,
        holder.host,
        holder.worktree,
        format_age(now_secs().saturating_sub(holder.acquired)),
        holder.command
    )
}

/// `ops lock status [<name>]`, writer-injectable.
///
/// # Errors
///
/// A name is invalid, or a lock cannot be probed, or writing `out` failed.
pub fn status_to(dir: &Path, name: Option<&str>, out: &mut dyn Write) -> anyhow::Result<()> {
    let names = match name {
        Some(name) => vec![name.to_string()],
        None => lock_names(dir)?,
    };
    if names.is_empty() {
        writeln!(out, "No locks.").context("printing the lock status")?;
    }
    for name in &names {
        let line = match probe(dir, name)? {
            LockState::Free => format!("{name}: free"),
            LockState::Held(Some(holder)) => {
                format!("{name}: held (alive) — {}", describe(&holder))
            }
            LockState::Held(None) => format!("{name}: held (alive) — holder still recording"),
            LockState::Stale(holder) => format!(
                "{name}: stale (holder dead, lock free) — {}; clear with `ops lock break {name}`",
                describe(&holder)
            ),
        };
        writeln!(out, "{line}").context("printing the lock status")?;
    }
    Ok(())
}

/// `ops lock break <name>`: clear a dead holder's record.
///
/// A live holder cannot be broken: its lock is a kernel lock bound to the
/// holding process, so the only way to free it is to stop that process —
/// the error names it.
///
/// # Errors
///
/// The name is invalid, the lock is held by a live process, or the record
/// cannot be cleared.
pub fn break_to(dir: &Path, name: &str, out: &mut dyn Write) -> anyhow::Result<()> {
    match probe(dir, name)? {
        LockState::Free => {
            writeln!(out, "{name}: not held, nothing to break").context("printing")?;
        }
        LockState::Held(holder) => {
            let who = holder.map_or_else(|| "a live process".to_string(), |h| describe(&h));
            anyhow::bail!(
                "refusing to break {name}: held by {who} — the holder is alive; stop it \
                 and the lock is released"
            );
        }
        LockState::Stale(holder) => {
            let path = lock_path(dir, name);
            let mut file = open_lock_file(&path)?;
            match file.try_lock() {
                Ok(()) => {
                    write_record(&mut file, "")
                        .with_context(|| format!("clearing {}", path.display()))?;
                    file.unlock().ok();
                    writeln!(
                        out,
                        "{name}: cleared the stale record of {}",
                        describe(&holder)
                    )
                    .context("printing")?;
                }
                Err(_) => anyhow::bail!("{name} was taken while breaking it; re-run status"),
            }
        }
    }
    Ok(())
}

/// `ops lock <name> [--timeout SECS] -- <cmd…>`: hold the lock for the
/// command's lifetime and exit with the command's status.
///
/// # Errors
///
/// No command was given, the lock cannot be taken, or the command cannot be
/// spawned.
pub fn run_locked(
    cwd: &Path,
    name: &str,
    command: &[OsString],
    timeout: Option<Duration>,
) -> anyhow::Result<ExitCode> {
    let Some((program, args)) = command.split_first() else {
        anyhow::bail!("no command given: ops lock <name> -- <command…>");
    };
    let dir = locks_dir(cwd)?;
    let worktree = git_toplevel(cwd).unwrap_or_else(|| cwd.to_path_buf());
    let holder = Holder::current(&worktree, command);
    let guard = acquire(&dir, name, &holder, timeout, &mut std::io::stderr())?;
    let code = run_child(program, args)?;
    drop(guard);
    Ok(ExitCode::from(code))
}

fn git_toplevel(cwd: &Path) -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    output.status.success().then(|| {
        PathBuf::from(
            String::from_utf8_lossy(&output.stdout)
                .trim_end_matches(['\n', '\r'])
                .to_string(),
        )
    })
}

/// Run the child to completion, forwarding SIGINT/SIGTERM to it, and map
/// its end to an exit code: its own code, or `128 + signo` when it (or
/// `ops`) was signalled.
///
/// # Errors
///
/// The runtime cannot be built or the child cannot be spawned.
fn run_child(program: &OsString, args: &[OsString]) -> anyhow::Result<u8> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building the lock runtime")?;
    runtime.block_on(async {
        let mut child = tokio::process::Command::new(program)
            .args(args)
            .spawn()
            .with_context(|| format!("spawning {}", program.to_string_lossy()))?;
        let signalled = forward_signals_until_exit(&mut child).await?;
        Ok(signalled)
    })
}

#[cfg(unix)]
async fn forward_signals_until_exit(child: &mut tokio::process::Child) -> anyhow::Result<u8> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut term = signal(SignalKind::terminate()).context("installing the SIGTERM handler")?;
    let mut int = signal(SignalKind::interrupt()).context("installing the SIGINT handler")?;
    let mut received: Option<i32> = None;
    loop {
        tokio::select! {
            status = child.wait() => {
                let status = status.context("waiting for the locked command")?;
                return Ok(exit_code(status, received));
            }
            _ = term.recv() => {
                received = Some(libc::SIGTERM);
                forward(child, libc::SIGTERM);
            }
            _ = int.recv() => {
                received = Some(libc::SIGINT);
                forward(child, libc::SIGINT);
            }
        }
    }
}

#[cfg(not(unix))]
async fn forward_signals_until_exit(child: &mut tokio::process::Child) -> anyhow::Result<u8> {
    let status = child
        .wait()
        .await
        .context("waiting for the locked command")?;
    Ok(exit_code(status, None))
}

/// Send `signo` to the child, if it is still running.
#[cfg(unix)]
fn forward(child: &tokio::process::Child, signo: i32) {
    let Some(pid) = child.id().and_then(|pid| libc::pid_t::try_from(pid).ok()) else {
        return;
    };
    // SAFETY: `kill(2)` takes two plain integers and dereferences nothing.
    // `pid` is our own not-yet-reaped child (tokio reaps it only in
    // `wait`, which has not returned), so it cannot name a recycled pid.
    unsafe {
        libc::kill(pid, signo);
    }
}

/// The child's exit code; a signal death — the child's own, or `ops`'s
/// forwarded one — maps to the shell's `128 + signo`.
fn exit_code(status: std::process::ExitStatus, received: Option<i32>) -> u8 {
    let by_signal = {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt as _;
            status.signal().or(received)
        }
        #[cfg(not(unix))]
        {
            received
        }
    };
    if let Some(code) = status.code() {
        return u8::try_from(code).unwrap_or(1);
    }
    by_signal
        .and_then(|signo| u8::try_from(signo).ok())
        .and_then(|signo| signo.checked_add(128))
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holder(pid: u32) -> Holder {
        Holder {
            pid,
            host: "h".to_string(),
            worktree: "/wt".to_string(),
            command: "ops verify".to_string(),
            acquired: now_secs(),
        }
    }

    fn take(dir: &Path, name: &str) -> anyhow::Result<Guard> {
        acquire(
            dir,
            name,
            &holder(std::process::id()),
            Some(Duration::from_millis(300)),
            &mut std::io::sink(),
        )
    }

    fn status(dir: &Path, name: Option<&str>) -> String {
        let mut out = Vec::new();
        status_to(dir, name, &mut out).expect("status");
        String::from_utf8(out).expect("utf8")
    }

    /// Contention: a second acquirer times out naming the holder, and gets
    /// the lock once the first releases. (`flock` locks belong to an open
    /// file description, so two handles in one process contend exactly like
    /// two processes.)
    #[test]
    fn a_held_lock_blocks_a_second_acquirer_until_released() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = take(dir.path(), "merge").expect("first");
        let err = take(dir.path(), "merge").expect_err("contended");
        assert!(format!("{err:#}").contains("timed out"), "got: {err:#}");
        assert!(
            format!("{err:#}").contains("ops verify"),
            "names the holder"
        );
        assert!(matches!(
            probe(dir.path(), "merge").expect("probe"),
            LockState::Held(Some(_))
        ));
        let text = status(dir.path(), Some("merge"));
        assert!(text.contains("held (alive)"), "got: {text}");
        assert!(text.contains("worktree /wt"), "got: {text}");
        drop(first);
        take(dir.path(), "merge").expect("free after release");
    }

    /// A stale holder: a record left behind with the lock free (the holder
    /// was killed) is reported stale, does not block a new acquirer, and can
    /// be broken.
    #[test]
    fn a_dead_holders_record_is_stale_and_does_not_block() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            lock_path(dir.path(), "merge"),
            holder(u32::MAX - 1).render(),
        )
        .expect("seed");
        assert!(matches!(
            probe(dir.path(), "merge").expect("probe"),
            LockState::Stale(_)
        ));
        let text = status(dir.path(), None);
        assert!(text.contains("merge: stale"), "got: {text}");

        let mut out = Vec::new();
        break_to(dir.path(), "merge", &mut out).expect("break");
        assert_eq!(probe(dir.path(), "merge").expect("probe"), LockState::Free);

        std::fs::write(
            lock_path(dir.path(), "merge"),
            holder(u32::MAX - 1).render(),
        )
        .expect("reseed");
        take(dir.path(), "merge").expect("a stale record never blocks");
    }

    /// A live holder cannot be broken.
    #[test]
    fn break_refuses_a_live_holder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _held = take(dir.path(), "merge").expect("take");
        let err = break_to(dir.path(), "merge", &mut Vec::new()).expect_err("live");
        assert!(format!("{err:#}").contains("refusing"), "got: {err:#}");
    }

    /// Release on failure: a guard dropped on an error path (here the
    /// normal drop after a failing command) frees the lock and clears the
    /// record.
    #[test]
    fn release_clears_the_record_and_frees_the_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result: anyhow::Result<()> = (|| {
            let _guard = take(dir.path(), "merge")?;
            anyhow::bail!("the locked step failed")
        })();
        assert!(result.is_err());
        assert_eq!(probe(dir.path(), "merge").expect("probe"), LockState::Free);
        assert_eq!(
            std::fs::read_to_string(lock_path(dir.path(), "merge")).expect("read"),
            ""
        );
    }

    #[test]
    fn names_are_restricted_to_a_safe_alphabet() {
        for bad in ["", "../x", "a/b", ".hidden", "-x", "a b"] {
            assert!(validate_name(bad).is_err(), "{bad:?} accepted");
        }
        for good in ["merge", "code-review.backlog", "wave_1"] {
            validate_name(good).expect(good);
        }
    }

    #[test]
    fn record_round_trips_and_flattens_newlines() {
        let mut h = holder(42);
        h.command = "sh -c 'a\npid=1'".to_string();
        let parsed = Holder::parse(&h.render()).expect("parse");
        assert_eq!(parsed.pid, 42);
        assert_eq!(parsed.command, "sh -c 'a pid=1'");
        assert_eq!(Holder::parse(""), None);
    }

    #[test]
    fn age_formats() {
        assert_eq!(format_age(5), "5s");
        assert_eq!(format_age(184), "3m04s");
        assert_eq!(format_age(7500), "2h05m");
    }
}
