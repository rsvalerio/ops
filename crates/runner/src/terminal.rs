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

            // SAFETY: tcgetattr reads terminal attributes into the provided buffer.
            // fd is a valid file descriptor (stderr) and termios is properly aligned.
            let ret = unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) };
            if ret != 0 {
                tracing::debug!("tcgetattr failed, skipping echo suppression");
                return Self { original: None };
            }

            // SAFETY: tcgetattr succeeded, so termios is fully initialized.
            let original = unsafe { termios.assume_init() };
            let mut modified = original;
            modified.c_lflag &= !libc::ECHO;

            // SAFETY: tcsetattr applies terminal attributes. TCSANOW applies immediately.
            // The modified struct is a valid copy of the original with only ECHO cleared.
            let ret = unsafe { libc::tcsetattr(fd, libc::TCSANOW, &modified) };
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

impl Drop for EchoGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            if let Some(ref original) = self.original {
                let fd = libc::STDERR_FILENO;
                // SAFETY: restoring the original termios state saved in `disable_echo`.
                unsafe {
                    libc::tcsetattr(fd, libc::TCSANOW, original);
                }
            }
        }
    }
}
