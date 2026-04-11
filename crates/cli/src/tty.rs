//! Shared TTY utilities for interactive CLI commands.

use std::io::IsTerminal;

/// Check whether stdout is connected to a terminal.
pub fn is_stdout_tty() -> bool {
    std::io::stdout().is_terminal()
}

/// Bail with an error if stdout is not a terminal.
pub fn require_tty(cmd_name: &str) -> anyhow::Result<()> {
    require_tty_with(cmd_name, is_stdout_tty)
}

/// Testable variant that accepts an injectable TTY check.
pub fn require_tty_with<F: FnOnce() -> bool>(cmd_name: &str, is_tty: F) -> anyhow::Result<()> {
    if !is_tty() {
        anyhow::bail!("{cmd_name} requires an interactive terminal");
    }
    Ok(())
}

/// A name+description pair for use with `inquire::Select` / `inquire::MultiSelect`.
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

    #[test]
    fn select_option_display() {
        let opt = SelectOption {
            name: "build".to_string(),
            description: "Run cargo build".to_string(),
        };
        assert_eq!(format!("{opt}"), "build — Run cargo build");
    }
}
