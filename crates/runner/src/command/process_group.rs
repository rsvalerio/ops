//! CONC-9 / TASK-1919: process-group ownership for captured spawns.
//!
//! `kill_on_drop(true)` — the runner's original and only cancellation
//! mechanism — sends `SIGKILL` to the **direct child pid**. Every step this
//! tool runs is a program that itself forks (`cargo build` → `rustc` × N,
//! `npm run` → node → subprocesses, `sh -c "…"` → whatever it launched), so
//! killing the direct child leaves the expensive part of the tree running,
//! unowned and unreaped, after a timeout or a `fail_fast` cancellation.
//!
//! The fix is to own the tree. Every captured spawn is made its own process
//! **group leader** ([`configure_process_group`]), and cancellation signals
//! the whole group through [`ChildGroup`] rather than the leader alone:
//! `SIGTERM` first — so `cargo` can release its `.cargo-lock` and `docker`
//! its layers, which a bare `SIGKILL` never allows — then `SIGKILL` after a
//! bounded grace period.
//!
//! # Why the captured path only
//!
//! Raw steps ([`super::exec::exec_command_raw`]) deliberately stay in the
//! runner's own process group: the tty delivers `SIGINT` to the foreground
//! process group, so moving an interactive child out of it would stop
//! Ctrl-C from reaching it. Captured children have no tty interaction at all
//! (their stdin is `/dev/null`, ASYNC-6 / TASK-1918), so nothing is lost by
//! giving them their own group.
//!
//! # Non-unix
//!
//! Process groups are a unix concept. On other targets every function here
//! is a no-op and `kill_on_drop(true)` (set in
//! [`super::build::build_command_with`]) remains the cancellation mechanism,
//! with its documented limitation of reaching only the direct child.

use std::time::Duration;

/// Grace period between the `SIGTERM` and the `SIGKILL` sent to a cancelled
/// step's process group. Long enough for a compiler or container build to
/// unwind its own temporary state, short enough that a cancelled run does
/// not feel wedged.
pub const GROUP_TERM_GRACE: Duration = Duration::from_secs(2);

/// Configure `cmd` so the spawned child becomes its own process-group
/// leader, making its whole descendant tree addressable via `killpg`.
///
/// Must be called before `spawn`.
#[cfg(unix)]
pub fn configure_process_group(cmd: &mut tokio::process::Command) {
    // `0` means "use the child's own pid as the group id", i.e. the child
    // is the group leader and every process it forks inherits the group.
    cmd.process_group(0);
}

/// No-op fallback: see the module docs on non-unix targets.
#[cfg(not(unix))]
pub fn configure_process_group(_cmd: &mut tokio::process::Command) {}

/// An armed handle on a spawned child's process group.
///
/// While armed, dropping the handle tears the group down (`SIGTERM`, then
/// `SIGKILL` after [`GROUP_TERM_GRACE`]). That is what makes the two
/// drop-driven cancellation paths — `tokio::time::timeout` dropping the
/// spawn future, and `JoinSet::abort_all` dropping the whole task — reach
/// the tree instead of only its root.
///
/// Call [`ChildGroup::disarm`] once the child has exited *and* its output
/// has been collected: at that point the step completed normally and any
/// surviving descendant was deliberately backgrounded by the step itself,
/// which is not the runner's to kill.
pub struct ChildGroup {
    #[cfg(unix)]
    pgid: Option<libc::pid_t>,
    armed: bool,
}

impl ChildGroup {
    /// Take ownership of `child`'s process group.
    ///
    /// The child was spawned with `process_group(0)`, so its group id equals
    /// its pid. A `None` pid means the child has already been reaped by an
    /// earlier `wait`, in which case there is no group left to own.
    #[cfg(unix)]
    pub fn new(child: &tokio::process::Child) -> Self {
        let pgid = child.id().and_then(|id| libc::pid_t::try_from(id).ok());
        Self {
            armed: pgid.is_some(),
            pgid,
        }
    }

    /// Non-unix: nothing to own; `kill_on_drop` is the fallback.
    #[cfg(not(unix))]
    pub fn new(_child: &tokio::process::Child) -> Self {
        Self { armed: false }
    }

    /// Stop the destructor from signalling the group. Use after a normal,
    /// fully-drained completion.
    pub const fn disarm(&mut self) {
        self.armed = false;
    }

    /// Send `signal` to the whole group. Returns `false` when there is no
    /// group to signal or the group is already gone (`ESRCH`).
    #[cfg(unix)]
    fn signal(&self, signal: libc::c_int) -> bool {
        let Some(pgid) = self.pgid else {
            return false;
        };
        // SAFETY: `killpg` is a plain libc call with no memory operands.
        // `pgid` is the pid of a child this process spawned as a group
        // leader; Linux keeps a pid reserved while it is still in use as a
        // process-group id, so the id cannot have been recycled by an
        // unrelated group while any member of ours survives. A group that
        // has fully exited yields `ESRCH`, which is reported as `false`
        // rather than treated as an error.
        let ret = unsafe { libc::killpg(pgid, signal) };
        ret == 0
    }

    /// Immediately `SIGKILL` the whole group, without the `SIGTERM` grace.
    ///
    /// Used on the post-exit drain deadline: the leader has already exited,
    /// so anything still holding the inherited pipe write end is an orphan
    /// that will otherwise block the step forever.
    #[cfg(unix)]
    pub fn kill_now(&self) -> bool {
        self.signal(libc::SIGKILL)
    }

    /// Non-unix: no group to kill.
    #[cfg(not(unix))]
    pub fn kill_now(&self) -> bool {
        false
    }
}

impl Drop for ChildGroup {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        #[cfg(unix)]
        {
            if !self.signal(libc::SIGTERM) {
                return;
            }
            let Some(pgid) = self.pgid else { return };
            // The escalation cannot be awaited: this runs in `Drop`, on the
            // cancellation path, where the surrounding task has already been
            // aborted and no future of ours will be polled again. A detached
            // OS thread is therefore the only carrier that survives long
            // enough to deliver the `SIGKILL`, and it is independent of the
            // tokio runtime, which may itself be shutting down. It lives for
            // `GROUP_TERM_GRACE` and only on the (rare) cancellation path.
            std::thread::spawn(move || {
                std::thread::sleep(GROUP_TERM_GRACE);
                // SAFETY: see `ChildGroup::signal` — same call, same
                // reservation argument for `pgid`.
                unsafe { libc::killpg(pgid, libc::SIGKILL) };
            });
        }
    }
}
