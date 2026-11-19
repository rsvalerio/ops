//! The process-wide spawn lock.
//!
//! On targets without `pipe2` (macOS among them) `std` creates a child's
//! capture pipes with `pipe()` and only then sets `FD_CLOEXEC` on them. A
//! child spawned on another thread inside that window inherits the write
//! end, and so do all of *its* descendants. The pipe then reaches EOF only
//! once that unrelated holder exits: a 0.02 s `ops check-json` step stayed
//! "open" for as long as a concurrently spawned `cargo build` ran, and the
//! holder sat outside the step's process group, where killing the group
//! could not reach it.
//!
//! Every spawn in the workspace that creates pipes takes [`spawn_locked`]
//! around the spawn call itself, so no two of them can overlap that window.
//! The lock covers the spawn only, never the wait: holding it across a
//! child's lifetime would serialize the children themselves. On `pipe2`
//! targets (Linux) pipe creation is already atomic and the lock costs one
//! uncontended acquire per spawn.

use std::io;
use std::process::{Command, Output, Stdio};
use std::sync::Mutex;

/// The lock [`spawn_locked`] holds. One per process: every copy of this
/// crate linked into a binary is the same version, and a second copy would
/// only reopen the window between the two.
static SPAWN_LOCK: Mutex<()> = Mutex::new(());

/// Run `spawn` while holding the process-wide spawn lock.
///
/// `spawn` should do nothing but spawn the child: everything it does runs
/// under the lock and delays every other spawn in the process.
///
/// # Blocking
///
/// Waits for the lock on the calling thread. Async callers must call this
/// from `tokio::task::spawn_blocking`, never on a runtime worker thread.
///
/// # Panics
///
/// Panics when the lock is poisoned, under the shared
/// [`crate::sync::lock`] policy. The guarded section is a single spawn
/// call, so a poisoned lock means a spawn panicked.
pub fn spawn_locked<T>(spawn: impl FnOnce() -> T) -> T {
    let _serialized = crate::sync::lock(&SPAWN_LOCK, "spawn serialization lock");
    spawn()
}

/// [`Command::output`] with the spawn taken under [`spawn_locked`].
///
/// Pins stdin to `/dev/null` and pipes stdout and stderr, which is what
/// `output()` does for a command that set none of them. A caller that needs
/// a different wiring (stderr inherited, say) sets it and calls
/// `spawn_locked(|| cmd.spawn())` followed by `wait_with_output` itself.
///
/// # Blocking
///
/// Blocks until the child exits, like `output()`; see [`spawn_locked`] for
/// async callers.
///
/// # Errors
///
/// The child cannot be spawned, or reading its output or waiting for it
/// fails.
pub fn output_locked(cmd: &mut Command) -> io::Result<Output> {
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    spawn_locked(|| cmd.spawn())?.wait_with_output()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_locked_returns_the_closure_value() {
        assert_eq!(spawn_locked(|| 7), 7);
    }

    /// The lock is released when the closure returns, so a second spawn on
    /// another thread is not left waiting.
    #[test]
    fn spawn_locked_releases_the_lock_after_the_spawn() {
        spawn_locked(|| ());
        let joined = std::thread::spawn(|| spawn_locked(|| 1)).join();
        assert_eq!(joined.expect("second spawn must not panic"), 1);
    }

    #[cfg(unix)]
    #[test]
    fn output_locked_captures_both_streams_and_the_status() {
        let out = output_locked(Command::new("sh").args(["-c", "echo out; echo err >&2; exit 3"]))
            .expect("sh must spawn");
        assert_eq!(out.stdout, b"out\n");
        assert_eq!(out.stderr, b"err\n");
        assert_eq!(out.status.code(), Some(3));
    }

    /// stdin is `/dev/null`, so a child that reads it sees EOF at once
    /// instead of blocking on the test runner's terminal.
    #[cfg(unix)]
    #[test]
    fn output_locked_gives_the_child_an_empty_stdin() {
        let out = output_locked(Command::new("sh").args(["-c", "cat; echo done"]))
            .expect("sh must spawn");
        assert_eq!(out.stdout, b"done\n");
    }
}
