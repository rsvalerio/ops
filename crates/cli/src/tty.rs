//! Shared TTY utilities for interactive CLI commands.

use std::io::IsTerminal;

/// Check whether stdout is connected to a terminal.
///
/// Use this only to decide how to *render to stdout* (colour, table width).
/// It is the wrong predicate for gating a prompt: `inquire` never touches
/// stdout — see [`is_prompt_tty`].
pub fn is_stdout_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Check whether an interactive prompt can be rendered and answered.
///
/// `inquire` builds its terminal from **stderr** (its crossterm backend uses
/// `IO::Std(stderr())`, the console backend `Term::stderr()`) and reads key
/// events from the controlling terminal — stdin. stdout is the one stream it
/// never touches, so gating on [`is_stdout_tty`] is wrong in both directions:
/// `ops theme select > out.txt` would be refused even though the picker would
/// work, and `ops theme select 2>/dev/null` would be allowed and then render
/// the picker into `/dev/null`, leaving an apparently-hung terminal.
///
/// Both streams are required: stderr to draw the prompt, stdin to answer it.
pub fn is_prompt_tty() -> bool {
    std::io::stderr().is_terminal() && std::io::stdin().is_terminal()
}

/// Bail with an error if an interactive prompt cannot be rendered and answered.
pub fn require_tty(cmd_name: &str) -> anyhow::Result<()> {
    require_tty_with(cmd_name, is_prompt_tty)
}

/// Testable variant that accepts an injectable prompt-capability check.
pub fn require_tty_with<F: FnOnce() -> bool>(cmd_name: &str, is_tty: F) -> anyhow::Result<()> {
    if !is_tty() {
        anyhow::bail!("{cmd_name} requires an interactive terminal");
    }
    Ok(())
}

/// A name+description pair for use with `inquire::Select` / `inquire::MultiSelect`.
#[derive(Debug)]
pub struct SelectOption {
    pub name: String,
    pub description: String,
}

impl std::fmt::Display for SelectOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.name, self.description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_tty_fails_when_not_tty() {
        let result = require_tty_with("test-cmd", || false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
    }

    #[test]
    fn require_tty_succeeds_when_tty() {
        let result = require_tty_with("test-cmd", || true);
        assert!(result.is_ok());
    }

    /// Stand-in for the process's stream states, so a test can express
    /// "stdout is a terminal but stderr is not" without touching the real
    /// process. `prompt_gate` is the predicate `require_tty` uses in
    /// production (`is_prompt_tty`), expressed over the fake streams.
    struct Streams {
        stdout: bool,
        stderr: bool,
        stdin: bool,
    }

    impl Streams {
        fn prompt_gate(&self) -> bool {
            self.stderr && self.stdin
        }
    }

    #[test]
    fn prompt_gate_refuses_when_prompt_stream_is_not_a_terminal_but_stdout_is() {
        // `ops theme select 2>/dev/null`: the old stdout-based gate let this
        // through and inquire drew the picker into /dev/null.
        let streams = Streams {
            stdout: true,
            stderr: false,
            stdin: true,
        };
        assert!(streams.stdout, "stdout is a terminal in this scenario");
        let result = require_tty_with("theme select", || streams.prompt_gate());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("interactive terminal"));
    }

    #[test]
    fn prompt_gate_permits_when_prompt_stream_is_a_terminal_but_stdout_is_not() {
        // `ops theme select > out.txt`: the terminal is fully attached, so the
        // picker works and the command must not be refused.
        let streams = Streams {
            stdout: false,
            stderr: true,
            stdin: true,
        };
        assert!(!streams.stdout, "stdout is redirected in this scenario");
        assert!(require_tty_with("theme select", || streams.prompt_gate()).is_ok());
    }

    #[test]
    fn prompt_gate_refuses_when_stdin_is_not_a_terminal() {
        // A prompt that can be drawn but never answered is still a hang.
        let streams = Streams {
            stdout: true,
            stderr: true,
            stdin: false,
        };
        assert!(require_tty_with("theme select", || streams.prompt_gate()).is_err());
    }

    #[test]
    fn select_option_display() {
        let opt = SelectOption {
            name: "build".to_string(),
            description: "Run cargo build".to_string(),
        };
        assert_eq!(format!("{opt}"), "build — Run cargo build");
    }
}
