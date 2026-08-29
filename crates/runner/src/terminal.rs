//! Terminal echo control for progress display.
//!
//! Disables terminal echo while commands are running to prevent keystrokes
//! from corrupting `indicatif`'s progress bar output.

use std::io::IsTerminal;

/// RAII guard that disables terminal echo on creation and restores it on drop.
///
/// When `indicatif` renders progress bars, stray keystrokes cause the terminal
/// to echo characters into stderr, disrupting cursor tracking and producing
/// duplicate output lines. This guard suppresses echo for the duration of
/// command execution.
///
/// On non-TTY stderr or if termios operations fail, the guard is a no-op.
///
/// # Which termination paths run this guard's `Drop` (CONC-14 / TASK-1932)
///
/// | Path | `Drop` runs? |
/// |---|---|
/// | Normal return / early `return` / `?` | yes |
/// | Panic (unwinding) | yes |
/// | `SIGINT` (Ctrl-C) or `SIGTERM` during a run | **only via the CLI's shutdown path** |
/// | A *second* `SIGINT`/`SIGTERM` after that path was entered | no (CONC-14 / TASK-2023) |
/// | `panic = "abort"`, `SIGKILL`, `std::process::exit` | no |
///
/// A signal does not run destructors. `ops` therefore does not rely on
/// `Drop` alone for the interruption users actually perform: the CLI races
/// the plan against `SIGTERM`/`SIGINT` (`run_cmd::run_until_signal`) and
/// drops this guard on that path before returning `128 + signo`, so a
/// Ctrl-C'd run still leaves the terminal with `ECHO` restored. Anything
/// that bypasses unwinding entirely (`SIGKILL`, an abort) is outside what
/// any RAII guard can cover — the shell's own `reset` is the remedy there.
///
/// CONC-14 / TASK-2023 deliberately adds one more row to that last group.
/// Once the shutdown path has been entered, `run_until_signal` restores the
/// default disposition for both signals so a second Ctrl-C can still escape
/// a wedged teardown. That second signal kills the process without
/// unwinding, so this guard's `Drop` does not run and `ECHO` stays cleared —
/// accepted as the price of keeping the escape hatch users had before
/// TASK-1932, with `reset` as the remedy exactly as for `SIGKILL`.
///
/// TEST-5: Platform-specific terminal control via libc termios. Tested manually;
/// unit tests would require a PTY or mock which exceeds the complexity budget
/// for ~60 lines of platform-specific code.
pub struct EchoGuard {
    #[cfg(unix)]
    original: Option<libc::termios>,
}

impl EchoGuard {
    /// Disable echo on stderr's terminal. Returns a guard that restores echo on drop.
    pub fn disable_echo() -> Self {
        #[cfg(unix)]
        {
            if !std::io::stderr().is_terminal() {
                return Self { original: None };
            }

            let fd = libc::STDERR_FILENO;
            let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();

            // SAFETY:
            // - `fd` is STDERR_FILENO, a stable-valid FD per POSIX.
            // - `termios.as_mut_ptr()` is non-null, properly aligned for `libc::termios`
            //   (MaybeUninit guarantees alignment) and points to writable storage
            //   owned by this stack frame for the full call.
            // - `tcgetattr` writes a complete `libc::termios` on success (ret == 0);
            //   on failure the memory stays uninit and we only `assume_init` below
            //   in the success branch.
            let ret = unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) };
            if ret != 0 {
                tracing::debug!("tcgetattr failed, skipping echo suppression");
                return Self { original: None };
            }

            // SAFETY: `tcgetattr` returned 0 (checked above), which per POSIX
            // means every field of the termios struct was written. The `MaybeUninit`
            // invariant (all bits initialized before `assume_init`) is therefore
            // satisfied. If this is ever reached without the preceding ret == 0
            // check, reading padding bytes would be UB.
            let original = unsafe { termios.assume_init() };
            let mut modified = original;
            modified.c_lflag &= !libc::ECHO;

            // SAFETY:
            // - `fd` is STDERR_FILENO.
            // - `&modified` points to a fully-initialized `libc::termios` (copy of
            //   `original` with only the ECHO bit cleared in c_lflag); all other
            //   fields, including any opaque/padding c_cc and speed fields, are
            //   preserved byte-for-byte. `tcsetattr` only reads from this pointer.
            // - TCSANOW applies immediately without draining pending output, which
            //   is what we want while a progress bar is already on screen.
            let ret = unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw const modified) };
            if ret != 0 {
                tracing::debug!("tcsetattr failed, skipping echo suppression");
                return Self { original: None };
            }

            Self {
                original: Some(original),
            }
        }

        #[cfg(not(unix))]
        {
            Self {}
        }
    }
}

impl EchoGuard {
    /// True when the guard is a no-op (non-TTY stderr or termios failure).
    /// Exposed for tests; external callers have no reason to inspect guard state.
    #[cfg(test)]
    pub(crate) const fn is_noop(&self) -> bool {
        #[cfg(unix)]
        {
            self.original.is_none()
        }
        #[cfg(not(unix))]
        {
            true
        }
    }
}

impl Drop for EchoGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if let Some(ref original) = self.original {
                let fd = libc::STDERR_FILENO;
                // SAFETY:
                // - `fd` is STDERR_FILENO; it was a TTY when `disable_echo` ran.
                //   If stderr has since been redirected, `tcsetattr` will fail
                //   cleanly with EBADF/ENOTTY — no UB, just a no-op restore.
                // - `original` is a valid `libc::termios` saved from a successful
                //   `tcgetattr` on the same fd, so every field matches the
                //   platform's current ABI. `tcsetattr` only reads through the
                //   pointer; no aliasing or writes occur.
                // - Ignoring the return is intentional: if restore fails during
                //   drop we have no recovery channel, and the terminal will be
                //   reset on process exit regardless.
                unsafe {
                    libc::tcsetattr(fd, libc::TCSANOW, original);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Non-TTY coverage: under `cargo test`, stderr is captured by the test
    /// harness (a pipe), so `disable_echo` must take the early-return branch
    /// and produce a no-op guard. Exercises the only branch we can reach
    /// without a PTY harness.
    #[test]
    fn disable_echo_is_noop_when_stderr_is_not_a_tty() {
        if std::io::stderr().is_terminal() {
            return;
        }
        let guard = EchoGuard::disable_echo();
        assert!(
            guard.is_noop(),
            "non-TTY stderr should yield a no-op EchoGuard"
        );
        drop(guard);
    }
}
