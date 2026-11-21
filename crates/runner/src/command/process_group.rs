//! Process-group ownership for captured and raw spawns.
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
//! # Raw steps: the foreground hand-off
//!
//! Raw steps ([`super::exec::exec_command_raw`]) get the same group
//! ownership through [`spawn_raw_group_and_wait`], plus the terminal
//! choreography a group of their own requires: a raw child must hold the
//! terminal's *foreground* process group for Ctrl-C / Ctrl-Z to reach it, so
//! the runner hands the foreground to the child for the duration of the step
//! (`tcsetpgrp`, with `SIGTTOU` neutralised), relays a Ctrl-C that killed the
//! child back onto itself so the run still cancels, implements just enough
//! job control to stop with a stopped child and continue it after the
//! shell's `fg` or `bg`, and takes the terminal back on every exit path —
//! a timeout's teardown included, synchronously. When the runner itself is
//! not the foreground group (`ops … &`), the child keeps the legacy shared
//! group and a timeout kills only it; with no controlling terminal at all,
//! the child gets its own group and nothing else — see [`raw_foreground`]'s
//! docs for the split.
//!
//! # Non-unix
//!
//! Process groups are a unix concept. On other targets every function here
//! is a no-op and `kill_on_drop(true)` (set in
//! [`super::build::build_command_with`]) remains the cancellation mechanism,
//! with its documented limitation of reaching only the direct child.
//!
//! # Miri coverage
//!
//! The `killpg` calls in this module cannot run under Miri: they are
//! direct foreign calls into libc, for which Miri provides no shims, and
//! the signals they deliver only mean something against a real process
//! table. The substitute evidence is the per-block
//! `// SAFETY:` prose (the pid-reservation argument for the pre-grace
//! signal; the probe-then-kill narrowing for the post-grace escalation)
//! plus the cancellation tests that run this code against real child
//! processes under the ordinary Test job. The runner crate's
//! Miri-runnable unsafe (the `EchoGuard` non-TTY construction path) is
//! covered by the `miri` job in `.github/workflows/ci.yml`; see
//! `terminal.rs`'s module docs for the termios exemption. The pty-driven
//! foreground tests in `tests/raw_mode.rs` likewise require a real terminal
//! and run only under the ordinary Test job.

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
        Self { armed: pgid.is_some(), pgid }
    }

    /// Non-unix: nothing to own; `kill_on_drop` is the fallback.
    #[cfg(not(unix))]
    pub fn new(_child: &tokio::process::Child) -> Self {
        Self { armed: false }
    }

    /// Take ownership of a process group by its id, armed for the
    /// `Drop` teardown.
    ///
    /// The raw path waits for its child on a dedicated thread with
    /// `waitpid(WUNTRACED)` (tokio's wait cannot report stops), so the group
    /// id arrives as a bare pid rather than inside a `tokio::process::Child`.
    /// The pid is a child this process spawned as a group leader, so the
    /// [`ChildGroup::new`] reservation argument carries over unchanged.
    #[cfg(unix)]
    #[must_use]
    pub const fn from_pgid(pgid: libc::pid_t) -> Self {
        Self { armed: true, pgid: Some(pgid) }
    }

    /// Stop the destructor from signalling the group. Use after a normal,
    /// fully-drained completion.
    pub const fn disarm(&mut self) {
        self.armed = false;
    }

    /// Send `signal` to the whole group. Returns `false` when there is no
    /// group to signal or the group is already gone (`ESRCH`).
    #[cfg(unix)]
    #[expect(unsafe_code, reason = "libc::killpg FFI; see the SAFETY comment")]
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
    #[expect(unsafe_code, reason = "libc::killpg FFI; see the SAFETY comments")]
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
                // SEC: unlike the pre-grace signal in `ChildGroup::signal`,
                // the pid reservation argument does *not* carry across the
                // sleep. The reservation only holds while the group still has
                // a member; if the `SIGTERM` above was obeyed and the leader
                // was reaped during the grace window, `pgid` is free and the
                // kernel may already have handed it to an unrelated process
                // group. Probe with signal 0 first and bail on `ESRCH` so the
                // escalation can only ever land on a group that is still
                // live.
                //
                // SAFETY: `killpg` is a plain libc call with no memory
                // operands; signal 0 performs the permission/existence check
                // without delivering anything.
                if unsafe { libc::killpg(pgid, 0) } != 0 {
                    return;
                }
                // SAFETY: same call as above. The probe narrows — but cannot
                // close — the reuse window: the group could still exit and
                // its pgid be recycled between the probe and this line. That
                // residual race is inherent to signalling by pid and is the
                // same one every `kill`-after-check carries.
                unsafe { libc::killpg(pgid, libc::SIGKILL) };
            });
        }
    }
}

#[cfg(unix)]
pub use raw_foreground::spawn_raw_group_and_wait;

/// The raw-step foreground hand-off.
///
/// A raw step's child is spawned as its own process-group leader so a
/// timeout can tear the whole tree down through [`ChildGroup`] — the same
/// ownership the captured path has. Group leadership alone would break the
/// terminal contract, though: the tty delivers Ctrl-C / Ctrl-Z to its
/// *foreground* group, which a freshly created group is not. The three
/// terminal situations therefore get three different treatments, decided
/// before the spawn in `classify_terminal`:
///
/// - **The runner holds the foreground** (the normal interactive `ops …`
///   invocation): the child gets the terminal for the duration of the step
///   and the runner takes it back on every exit path — the "foreground
///   hand-off". The child claims the terminal itself between fork and exec
///   and the parent repeats the claim after the spawn, the way job-control
///   shells do, so the step's program never runs a single instruction
///   without the terminal. A Ctrl-C then reaches only the child's group, so
///   after a child that died of `SIGINT` the wait re-raises `SIGINT` on the
///   runner itself, driving the same run-level cancellation the
///   shared-group design got for free. Whenever the child's group stops —
///   a Ctrl-Z, or a terminal read after the user `bg`'d the run — the
///   runner stops with it the way a job-control job does, so the user's
///   shell sees the whole run stopped, and on resume the child is continued,
///   with the terminal when the resume brought it (`fg`) and without it
///   when it did not (`bg`).
/// - **No controlling terminal** (piped stdio, CI): nothing to hand over or
///   stop, but also nothing that can break — the child keeps its own group
///   and a timeout tears the tree down. This is the case a raw step runs in
///   under the test harness.
/// - **Another group holds the foreground** (`ops … &`): the runner may not
///   take the terminal from the shell, and a child in its own group would
///   only earn itself `SIGTTIN`/`SIGTTOU` stops on terminal reads/writes,
///   so the child stays in the runner's group. `killpg` cannot single that
///   child out, so a timeout `SIGKILL`s the direct child only — the legacy
///   `kill_on_drop` behaviour, with its documented limitation.
///
/// Terminal ownership lives in one `RawState`, shared under one lock by
/// the wait thread and the async side's `RawStepGuard`, so a hand-off and
/// a teardown can never interleave. A timeout's teardown takes the terminal
/// back synchronously, before the timed-out step even returns, so the next
/// step always classifies against a terminal the runner holds; and the wait
/// thread never hands the terminal to, or takes it back from, a step whose
/// teardown has already run.
///
/// Everything else runs on one dedicated blocking thread
/// ([`spawn_raw_group_and_wait`]): the `WUNTRACED` wait and the job-control
/// stops share one signal environment, and the async side's only job is to
/// arm the `RawStepGuard` whose `Drop` is the timeout/cancellation
/// teardown.
#[cfg(unix)]
mod raw_foreground {
    use super::{configure_process_group, ChildGroup};
    use std::io;
    use std::process::ExitStatus;
    use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

    /// How the raw child relates to the controlling terminal. Decided before
    /// the spawn, because it decides whether the child gets its own group.
    #[derive(Clone, Copy)]
    enum TerminalHandoff {
        /// The runner holds the terminal's foreground: hand it to the child
        /// for the duration of the step. The payload is an fd (one of
        /// 0/1/2) referring to the controlling terminal.
        Foreground(libc::c_int),
        /// No fd among 0/1/2 is a terminal (pipes, CI): the child leads its
        /// own group with no terminal to hand over. A timeout still tears
        /// the whole tree down; nothing can stop or interrupt.
        NoControllingTty,
        /// A terminal exists but its foreground is another group (`ops … &`):
        /// the child stays in the runner's group, because an own-group child
        /// would only collect `SIGTTIN`/`SIGTTOU` stops on terminal I/O.
        Background,
    }

    /// What the wait thread and the async side's guard share, under one
    /// lock.
    struct RawState {
        /// Set by the guard on timeout / run cancellation.
        cancelled: bool,
        /// The live child: set right after the spawn, cleared once the wait
        /// thread has reaped it or by the teardown, which therefore runs at
        /// most once.
        pid: Option<libc::pid_t>,
        /// How the child relates to the terminal; set together with `pid`.
        handoff: TerminalHandoff,
        /// Whether the child's group holds the terminal on the runner's
        /// behalf. Set when the runner hands the terminal over; cleared when
        /// the runner takes it back, and when the runner resumes from a stop
        /// without it (the user's `bg`). While set, the terminal is the
        /// runner's to reclaim; while clear it belongs to the user's shell
        /// and is never touched.
        holding_terminal: bool,
    }

    impl RawState {
        /// The timeout / cancellation teardown: take the terminal back if
        /// the child holds it for us, then signal the child. Runs at most
        /// once, because it takes `pid`.
        fn teardown(&mut self) {
            let Some(pid) = self.pid.take() else { return };
            if self.holding_terminal {
                if let TerminalHandoff::Foreground(fd) = self.handoff {
                    reclaim_terminal(fd);
                }
                self.holding_terminal = false;
            }
            match self.handoff {
                TerminalHandoff::Background => kill_direct_child(pid),
                // Constructed armed and dropped immediately: the Drop runs
                // the SIGTERM + delayed-SIGKILL escalation on the group.
                TerminalHandoff::Foreground(_) | TerminalHandoff::NoControllingTty => {
                    drop(ChildGroup::from_pgid(pid));
                }
            }
        }
    }

    struct RawShared(Mutex<RawState>);

    impl RawShared {
        const fn new() -> Self {
            Self(Mutex::new(RawState {
                cancelled: false,
                pid: None,
                handoff: TerminalHandoff::NoControllingTty,
                holding_terminal: false,
            }))
        }

        fn lock(&self) -> MutexGuard<'_, RawState> {
            // Every critical section is a handful of non-panicking syscalls,
            // so the lock cannot poison in practice; the fallback keeps a
            // panicking sibling thread from failing an unrelated step.
            self.0.lock().unwrap_or_else(PoisonError::into_inner)
        }
    }

    /// Armed by the async side for the lifetime of the raw-step future.
    ///
    /// Dropping it while armed is the timeout / run-cancellation teardown
    /// (`RawState::teardown`). The wait thread observes the dying child and
    /// exits without reporting. [`Self::disarm`] on a normal completion
    /// matches the captured path's contract: a descendant that outlives a
    /// *successfully completed* step was deliberately backgrounded by the
    /// step and is not the runner's to kill.
    struct RawStepGuard {
        shared: Arc<RawShared>,
        armed: bool,
    }

    impl RawStepGuard {
        const fn disarm(&mut self) {
            self.armed = false;
        }
    }

    impl Drop for RawStepGuard {
        fn drop(&mut self) {
            if !self.armed {
                return;
            }
            let mut state = self.shared.lock();
            state.cancelled = true;
            // No pid yet: the wait thread re-checks `cancelled` under this
            // lock right after the spawn, and runs the teardown itself.
            state.teardown();
        }
    }

    /// Decide how this raw step relates to the controlling terminal.
    ///
    /// The first of stdin/stdout/stderr that is a terminal identifies the
    /// controlling terminal; its foreground group decides the arm. No
    /// waiting is needed for a previous raw step's terminal to come back:
    /// every exit path of a raw step, the timeout teardown included, has
    /// returned the terminal before the step's future completes.
    #[expect(unsafe_code, reason = "tcgetpgrp/getpgrp FFI; see the SAFETY comment")]
    fn classify_terminal() -> TerminalHandoff {
        // SAFETY: getpgrp takes no arguments and only reads the caller's
        // process-group membership; there is no memory to validate.
        let ours = unsafe { libc::getpgrp() };
        for fd in [libc::STDIN_FILENO, libc::STDOUT_FILENO, libc::STDERR_FILENO] {
            // SAFETY: tcgetpgrp only inspects the descriptor's terminal; it
            // dereferences nothing, and a non-terminal fd is reported as -1.
            let fg = unsafe { libc::tcgetpgrp(fd) };
            if fg < 0 {
                continue;
            }
            return if fg == ours {
                TerminalHandoff::Foreground(fd)
            } else {
                TerminalHandoff::Background
            };
        }
        TerminalHandoff::NoControllingTty
    }

    /// A no-op `SIGTTOU` disposition, installed once per process.
    const extern "C" fn sigttou_noop(_signo: libc::c_int) {}

    /// Neutralise `SIGTTOU` for the whole process, once.
    ///
    /// Every `tcsetpgrp` here runs with `SIGTTOU` blocked on the calling
    /// thread ([`set_foreground`]), which is what lets a background caller
    /// change the foreground without the kernel raising `SIGTTOU` at all.
    /// The disposition is the backstop for anything that slips past a
    /// per-thread mask: the signal's default action stops the process —
    /// including tokio worker threads that have nothing to do with the
    /// hand-off. A *caught* disposition is also the safe kind to keep:
    /// `exec` resets caught dispositions to the default in the new image,
    /// so spawned children never inherit it (an `SIG_IGN` would survive
    /// `exec` and silently change a child's own terminal behaviour).
    fn neutralise_sigttou_once() {
        use std::sync::Once;
        static INSTALL: Once = Once::new();
        INSTALL.call_once(|| {
            #[expect(unsafe_code, reason = "libc::signal FFI; see the SAFETY comment")]
            // SAFETY: `signal(2)` rewrites only this process's SIGTTOU
            // disposition; the handler body is empty and therefore
            // async-signal-safe. The returned previous disposition is
            // deliberately discarded: the kernel default is what any
            // non-raw code path already runs with, and this install never
            // unwinds.
            //
            // The `as` cast is the sanctioned way to name a handler for
            // `sighandler_t` (a plain integer alias — no `From` impl exists
            // for fn pointers).
            #[expect(
                clippy::as_conversions,
                reason = "sighandler_t is an integer alias for the handler address"
            )]
            unsafe {
                libc::signal(libc::SIGTTOU, sigttou_noop as *const () as libc::sighandler_t);
            }
        });
    }

    /// Block the given signals on this thread, restoring the previous mask
    /// on drop.
    struct SigMaskGuard {
        previous: libc::sigset_t,
    }

    impl SigMaskGuard {
        #[expect(
            unsafe_code,
            reason = "sigset_t shaping and pthread_sigmask FFI; see the SAFETY comments"
        )]
        fn block(signals: &[libc::c_int]) -> Self {
            unsafe {
                let mut set: libc::sigset_t = std::mem::zeroed();
                // SAFETY: `set` is a fully-formed (zeroed) sigset_t owned by
                // this frame; sigemptyset/sigaddset only write through the
                // pointer.
                libc::sigemptyset(&raw mut set);
                for &signo in signals {
                    libc::sigaddset(&raw mut set, signo);
                }
                let mut previous: libc::sigset_t = std::mem::zeroed();
                // SAFETY: both pointers are fully-formed sigset_t values
                // owned by this frame; pthread_sigmask writes `previous`
                // completely on success.
                libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, &raw mut previous);
                Self { previous }
            }
        }
    }

    impl Drop for SigMaskGuard {
        #[expect(unsafe_code, reason = "pthread_sigmask FFI; see the SAFETY comment")]
        fn drop(&mut self) {
            // SAFETY: `previous` was written by the matching pthread_sigmask
            // call in `block` and only ever read here.
            unsafe {
                libc::pthread_sigmask(
                    libc::SIG_SETMASK,
                    &raw const self.previous,
                    std::ptr::null_mut(),
                );
            }
        }
    }

    /// Make `pgid` the foreground group of the terminal `fd` refers to.
    ///
    /// `SIGTTOU` is blocked on the calling thread for the call: a caller in
    /// a background group — the runner after it handed the terminal away,
    /// or a teardown running on a tokio worker — would otherwise have the
    /// kernel raise `SIGTTOU` and refuse the change. A blocked `SIGTTOU`
    /// counts as ignored, so the change goes through and nothing is raised.
    /// Returns whether the change succeeded.
    #[expect(unsafe_code, reason = "tcsetpgrp FFI; see the SAFETY comment")]
    fn set_foreground(fd: libc::c_int, pgid: libc::pid_t) -> bool {
        let _ttou_blocked = SigMaskGuard::block(&[libc::SIGTTOU]);
        // SAFETY: tcsetpgrp only writes the terminal's foreground-group
        // attribute; fd refers to our controlling terminal and `pgid` is a
        // group in our session.
        unsafe { libc::tcsetpgrp(fd, pgid) == 0 }
    }

    /// Give the terminal back to the runner's own group.
    #[expect(unsafe_code, reason = "getpgrp FFI; see the SAFETY comment")]
    fn reclaim_terminal(fd: libc::c_int) {
        // SAFETY: getpgrp takes no arguments and only reads kernel state.
        let ours = unsafe { libc::getpgrp() };
        if !set_foreground(fd, ours) {
            tracing::warn!(
                error = %io::Error::last_os_error(),
                "could not take the terminal back from the raw step"
            );
        }
    }

    /// `SIGKILL` the background arm's child. It shares the runner's group,
    /// so `killpg` cannot single it out; this is the legacy `kill_on_drop`
    /// behaviour, reaching the direct child only.
    #[expect(unsafe_code, reason = "libc::kill FFI; see the SAFETY comment")]
    fn kill_direct_child(pid: libc::pid_t) {
        // SAFETY: kill(2) has no memory operands. `pid` is a child of this
        // process that the wait thread has not reaped yet: the wait thread
        // clears the shared pid under the same lock right after reaping, so
        // the pid cannot have been recycled — short of the few instructions
        // between the kernel's reap and that clear, the residual every
        // signal-by-pid carries.
        unsafe {
            libc::kill(pid, libc::SIGKILL);
        }
    }

    /// Have the child claim the terminal itself, between fork and exec.
    ///
    /// The parent's `tcsetpgrp` lands only after the spawn returns, by
    /// which time the step's program is already running in a group that
    /// does not hold the terminal: a program that reads the terminal at
    /// once would stop on `SIGTTIN`, which the wait could not tell from a
    /// Ctrl-Z. Claiming in the child closes that window; the parent repeats
    /// the claim after the spawn — the way job-control shells hand the
    /// terminal to a new job — so the hand-off holds whichever side gets
    /// there first, and a failed child-side claim (say, a platform that
    /// sets up the process group after the pre-exec hook) is still covered.
    #[expect(
        unsafe_code,
        reason = "pre_exec hook with sigprocmask/tcsetpgrp FFI; see the SAFETY comment"
    )]
    fn claim_terminal_in_child(cmd: &mut tokio::process::Command, fd: libc::c_int) {
        // SAFETY: the hook runs in the forked child before exec, where only
        // async-signal-safe functions may be called. It calls sigemptyset,
        // sigaddset, sigprocmask, getpid and tcsetpgrp — all
        // async-signal-safe — on stack values it owns; it allocates nothing
        // and takes no lock. Blocking SIGTTOU lets the child, still in a
        // background group, change the foreground without stopping; the
        // previous mask is restored so the step's program starts with the
        // mask it would have had. A failed claim is deliberately ignored:
        // the parent's claim follows.
        unsafe {
            cmd.pre_exec(move || {
                let mut set: libc::sigset_t = std::mem::zeroed();
                libc::sigemptyset(&raw mut set);
                libc::sigaddset(&raw mut set, libc::SIGTTOU);
                let mut previous: libc::sigset_t = std::mem::zeroed();
                libc::sigprocmask(libc::SIG_BLOCK, &raw const set, &raw mut previous);
                libc::tcsetpgrp(fd, libc::getpid());
                libc::sigprocmask(libc::SIG_SETMASK, &raw const previous, std::ptr::null_mut());
                Ok(())
            });
        }
    }

    /// Stop this process the way a job-control shell's foreground job stops,
    /// and return once a `SIGCONT` resumed it.
    ///
    /// `SIGSTOP`, not a raised `SIGTSTP`: a stop signal cannot be blocked or
    /// intercepted here (so the runner stops even when it inherited an
    /// ignored `SIGTSTP` from a background launch), and — unlike the
    /// `SIGTSTP`/`SIGTTIN`/`SIGTTOU` family, whose default stop action
    /// POSIX *discards* for an orphaned process group — it stops even when
    /// the runner's group has no parent in its session to notice. The
    /// user's `fg`/`bg` resume a `SIGSTOP`'d job exactly like a `SIGTSTP`'d
    /// one: `SIGCONT`, plus the terminal for `fg`.
    #[expect(unsafe_code, reason = "raise FFI; see the SAFETY comment")]
    fn stop_self_until_cont() {
        // SAFETY: raise(2) sends the signal to the calling process only;
        // SIGSTOP's stop happens in the kernel and the call returns after
        // the SIGCONT resumes it. Nothing is dereferenced.
        unsafe {
            libc::raise(libc::SIGSTOP);
        }
    }

    /// The child's group stopped on the foreground arm: stop with it, and
    /// continue it on resume.
    ///
    /// This covers both ways the child can stop. On a Ctrl-Z the child held
    /// the terminal, so the runner takes it back before stopping, and the
    /// user's shell regains the prompt. After the user resumed the run with
    /// `bg`, the child no longer holds the terminal and a terminal read
    /// stops it on `SIGTTIN`; the runner then stops without touching the
    /// terminal — it is the shell's — so the shell reports the run stopped,
    /// exactly as it would for any background job that reads the terminal.
    /// Either way the user's next `fg` resumes the runner's own job with
    /// the terminal, and that resume is what continues the child — with the
    /// terminal handed back first, so it never runs without it.
    #[expect(unsafe_code, reason = "tcgetpgrp/getpgrp/killpg FFI; see the SAFETY comments")]
    fn stop_with_child(shared: &RawShared, pid: libc::pid_t, fd: libc::c_int) {
        {
            let mut state = shared.lock();
            if state.cancelled {
                // A teardown is in flight; its signals end the stop.
                return;
            }
            if state.holding_terminal {
                reclaim_terminal(fd);
                state.holding_terminal = false;
            }
        }
        stop_self_until_cont();
        {
            let mut state = shared.lock();
            if state.cancelled {
                return;
            }
            // SAFETY: getpgrp/tcgetpgrp only read kernel state.
            let resumed_with_terminal = unsafe { libc::tcgetpgrp(fd) == libc::getpgrp() };
            if resumed_with_terminal && set_foreground(fd, pid) {
                state.holding_terminal = true;
            }
        }
        // The terminal (if the resume brought it) is the child's before it
        // runs again. A teardown racing this continue is harmless: it only
        // helps the teardown's signals reach a group that would otherwise
        // sit stopped until the SIGKILL.
        //
        // SAFETY: killpg has no memory operands; `pid` leads a group this
        // process spawned and has not reaped. SIGCONT to a group that has
        // since died yields ESRCH, which is fine to ignore.
        unsafe {
            libc::killpg(pid, libc::SIGCONT);
        }
    }

    /// Wait for `pid` reporting stops as well as exits, running the
    /// job-control stop ([`stop_with_child`]) for the foreground arm.
    #[expect(unsafe_code, reason = "waitpid FFI; see the SAFETY comment")]
    fn wait_with_job_control(
        shared: &RawShared,
        pid: libc::pid_t,
        handoff: TerminalHandoff,
    ) -> io::Result<ExitStatus> {
        use std::os::unix::process::ExitStatusExt;
        loop {
            let mut raw: libc::c_int = 0;
            // SAFETY: waitpid writes a complete wait status through `raw`,
            // which is owned by this frame; the child is this process's own
            // and is never waited on by anyone else (tokio's reaper only
            // knows children it spawned itself).
            let waited = unsafe { libc::waitpid(pid, &raw mut raw, libc::WUNTRACED) };
            if waited == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    // Any of the runtime's SIGCHLDs; the wait simply resumes.
                    continue;
                }
                return Err(err);
            }
            if waited != pid {
                // A blocking waitpid without WNOHANG returns either the pid
                // or -1; anything else is not a status to act on.
                continue;
            }
            if libc::WIFSTOPPED(raw) {
                if let TerminalHandoff::Foreground(fd) = handoff {
                    stop_with_child(shared, pid, fd);
                }
                // The other arms have no terminal of the runner's to manage:
                // with no controlling terminal nothing stops the child but an
                // explicit signal, and a background-arm child shares the
                // runner's group, so the shell's job control reaches both.
                continue;
            }
            if libc::WIFEXITED(raw) || libc::WIFSIGNALED(raw) {
                // `raw` is a complete wait status for one child, as
                // documented for ExitStatusExt::from_raw.
                return Ok(ExitStatusExt::from_raw(raw));
            }
            // Without WCONTINUED only stops and terminal statuses can be
            // reported; anything else is a kernel oddity not worth acting
            // on, so keep waiting.
        }
    }

    /// The blocking side of [`spawn_raw_group_and_wait`]: classify the
    /// terminal, spawn under the process-wide spawn lock, hand the
    /// foreground over, wait with job control, take the terminal back, and
    /// relay a Ctrl-C that killed the child.
    #[expect(unsafe_code, reason = "libc FFI for the terminal hand-off; see the SAFETY comments")]
    fn spawn_and_wait_blocking(
        shared: &RawShared,
        cmd: &mut tokio::process::Command,
    ) -> io::Result<ExitStatus> {
        use std::os::unix::process::ExitStatusExt;
        neutralise_sigttou_once();
        let handoff = classify_terminal();
        match handoff {
            TerminalHandoff::Foreground(fd) => {
                configure_process_group(cmd);
                claim_terminal_in_child(cmd, fd);
            }
            TerminalHandoff::NoControllingTty => configure_process_group(cmd),
            TerminalHandoff::Background => {}
        }
        // spawn_child_locked is the sanctioned locked spawn; the child's
        // stdio is whatever exec_command_raw configured (all inherited).
        let child = match ops_core::subprocess::spawn_child_locked(cmd.as_std_mut()) {
            Ok(child) => child,
            Err(err) => {
                // An exec failure is reported after the child's own claim of
                // the terminal may have landed; the terminal was the
                // runner's before the spawn, so take it back.
                if let TerminalHandoff::Foreground(fd) = handoff {
                    reclaim_terminal(fd);
                }
                return Err(err);
            }
        };
        let pid = libc::pid_t::try_from(child.id())
            .map_err(|_| io::Error::other("raw child pid does not fit pid_t"))?;
        // Dropping the std Child is safe: it reaps nothing and kills
        // nothing; the waitpid below is the only reaper this pid has.
        drop(child);
        {
            let mut state = shared.lock();
            state.pid = Some(pid);
            state.handoff = handoff;
            // The child's own claim may already hold the terminal.
            state.holding_terminal = matches!(handoff, TerminalHandoff::Foreground(_));
            if state.cancelled {
                // The future was dropped between the spawn and this lock, so
                // the guard never saw a pid. Tear the child down here and
                // reap it, so no zombie and no orphan survive the cancelled
                // step.
                state.teardown();
                drop(state);
                // SAFETY: reaping our own child; a null status pointer is
                // allowed and nothing else is written.
                unsafe { libc::waitpid(pid, std::ptr::null_mut(), 0) };
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "raw step cancelled before its wait began",
                ));
            }
            if let TerminalHandoff::Foreground(fd) = handoff {
                // The parent half of the hand-off, only if the child's own
                // claim has not landed yet: once the child holds the
                // terminal it may already have passed it on to a job of its
                // own, which this must not take back.
                //
                // SAFETY: getpgrp/tcgetpgrp only read kernel state.
                let still_ours = unsafe { libc::tcgetpgrp(fd) == libc::getpgrp() };
                if still_ours && !set_foreground(fd, pid) {
                    tracing::warn!(
                        error = %io::Error::last_os_error(),
                        "could not hand the terminal to the raw step; \
                         it will stop if it reads the terminal"
                    );
                }
            }
        }
        let waited = wait_with_job_control(shared, pid, handoff);
        let cancelled = {
            let mut state = shared.lock();
            if waited.is_ok() {
                // Reaped: the pid is no longer this process's to signal.
                state.pid = None;
            }
            // Every exit path lands here: take the terminal back before the
            // status escapes, so the next step (or the shell, once the run
            // ends) never fights a dead group for the foreground. A
            // cancelled step's teardown already took it back, and must not
            // have it taken again from whatever runs next.
            if state.holding_terminal {
                if let TerminalHandoff::Foreground(fd) = handoff {
                    reclaim_terminal(fd);
                }
                state.holding_terminal = false;
            }
            state.cancelled
        };
        let status = waited?;
        // The Ctrl-C relay. While the child held the foreground, a terminal
        // Ctrl-C reached only its group; without this re-raise the runner's
        // cancellation path would never learn about it and the run would
        // continue. Re-raising on ourselves reaches exactly the handler the
        // shared-group design used to hit directly (the run-level signal
        // race, or the kernel default when no handler is installed).
        if !cancelled
            && matches!(handoff, TerminalHandoff::Foreground(_))
            && status.signal() == Some(libc::SIGINT)
        {
            // SAFETY: kill(2) to ourselves; the signal is SIGINT and the
            // disposition is whatever the process already runs with.
            unsafe { libc::kill(libc::getpid(), libc::SIGINT) };
        }
        Ok(status)
    }

    /// Spawn a raw step's `cmd` and wait for it, owning the child's whole
    /// process group.
    ///
    /// On unix the child is spawned as its own process-group leader; when
    /// the runner holds the terminal's foreground, the child is handed the
    /// foreground for the duration of the step and the terminal returns on
    /// every exit path (see the [module docs](self)). Dropping the returned
    /// future is the timeout / cancellation path: the terminal comes back
    /// at once, and the group is torn down through [`ChildGroup`]'s
    /// `SIGTERM` → grace → `SIGKILL` escalation, so a timed-out raw step
    /// leaves no descendants running. When the runner is a background job
    /// the child shares the runner's group and only it is killed.
    ///
    /// # Errors
    ///
    /// Returns the spawn error when the child cannot be spawned, or the
    /// `waitpid` error when the wait itself fails. A step killed by a signal
    /// (including a Ctrl-C relayed from the terminal) is *not* an error
    /// here; it is reported through the returned `ExitStatus`.
    pub async fn spawn_raw_group_and_wait(
        cmd: &mut tokio::process::Command,
    ) -> io::Result<ExitStatus> {
        let mut owned = std::mem::replace(cmd, tokio::process::Command::new(""));
        let shared = Arc::new(RawShared::new());
        let thread_shared = Arc::clone(&shared);
        let join = tokio::task::spawn_blocking(move || {
            let result = spawn_and_wait_blocking(&thread_shared, &mut owned);
            (owned, result)
        });
        let mut guard = RawStepGuard { shared, armed: true };
        match join.await {
            Ok((owned, result)) => {
                *cmd = owned;
                if result.is_ok() {
                    // The child was reaped: any surviving group member was
                    // deliberately backgrounded by a completed step.
                    guard.disarm();
                }
                result
            }
            Err(join_err) => {
                // The wait thread panicked; the guard stays armed and its
                // Drop tears the child down from the published pid and takes
                // the terminal back. The owned command is lost with the
                // thread, same as a panicking spawn in spawn_locked_on_pool.
                tracing::error!(
                    error = %join_err,
                    "raw wait thread panicked; converting to step failure"
                );
                Err(io::Error::other("raw wait task failed"))
            }
        }
    }
}
