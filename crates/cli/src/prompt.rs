//! Shared handling of `inquire` prompt results.
//!
//! Every interactive command in this crate funnels its prompt result through
//! this module so "the user pressed Esc / Ctrl-C" is treated the same way
//! everywhere: exit [`SIGINT_EXIT`] (130), no `ops: error:` frame. Before this
//! module existed the convention had three spellings and four prompt sites
//! that ignored it entirely, so `ops theme select` reported a cancel as
//! `ops: error: Operation was canceled by the user` and exit 1 — indistinguishable
//! from a real failure to any script wrapping the command.
//!
//! The `OperationCanceled | OperationInterrupted` match arm lives here and
//! nowhere else in the crate.

use anyhow::Context as _;

use crate::{ExitCodeOverride, SIGINT_EXIT};

/// Error marker for "the user backed out at a prompt".
///
/// `main` renders this as a plain `ops: note:` line and exits [`SIGINT_EXIT`],
/// rather than wrapping it in the `ops: error:` frame that a genuine failure
/// gets. It also carries [`ExitCodeOverride`] so any path that only inspects
/// the exit code still sees 130.
#[derive(Debug)]
pub struct PromptCancelled {
    prompt_source: String,
}

impl std::fmt::Display for PromptCancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} cancelled", self.prompt_source)
    }
}

impl std::error::Error for PromptCancelled {}

/// Build the error a command returns when the user cancels at a prompt.
pub fn cancelled(prompt_source: &str) -> anyhow::Error {
    anyhow::Error::new(PromptCancelled {
        prompt_source: prompt_source.to_string(),
    })
    .context(ExitCodeOverride(SIGINT_EXIT))
}

/// Find a [`PromptCancelled`] anywhere in an error chain.
///
/// Mirrors `main::extract_exit_code_override`: anyhow surfaces context values
/// via `downcast_ref` on the error itself, not via `chain()`, so both have to
/// be checked.
pub fn cancellation_of(err: &anyhow::Error) -> Option<&PromptCancelled> {
    if let Some(c) = err.downcast_ref::<PromptCancelled>() {
        return Some(c);
    }
    err.chain()
        .find_map(<dyn std::error::Error>::downcast_ref::<PromptCancelled>)
}

/// Classify an `inquire` prompt result, generic over the prompt's answer type
/// so `Confirm`, `Select`, `MultiSelect`, and `Text` all share one arm.
///
/// Returns `Ok(Some(answer))` for a real choice, `Ok(None)` for an explicit
/// cancel, and `Err` for any other inquire failure. The error branch attaches
/// a context naming the prompt so a `NotTTY` / IO failure tells the user which
/// prompt was in flight rather than surfacing a bare `inquire: <variant>` line.
pub fn classify_prompt_result<T>(
    res: Result<T, inquire::InquireError>,
    prompt_source: &str,
) -> anyhow::Result<Option<T>> {
    match res {
        Ok(answer) => Ok(Some(answer)),
        Err(
            inquire::InquireError::OperationCanceled | inquire::InquireError::OperationInterrupted,
        ) => Ok(None),
        Err(e) => {
            Err(anyhow::Error::new(e)).with_context(|| format!("{prompt_source} prompt failed"))
        }
    }
}

/// [`classify_prompt_result`] for callers that have nothing to do on cancel
/// except stop: a cancel becomes the shared [`cancelled`] error (exit 130, no
/// `ops: error:` frame) instead of `Ok(None)`.
pub fn require_answer<T>(
    res: Result<T, inquire::InquireError>,
    prompt_source: &str,
) -> anyhow::Result<T> {
    classify_prompt_result(res, prompt_source)?.ok_or_else(|| cancelled(prompt_source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_maps_a_real_answer_to_some() {
        let res: Result<bool, inquire::InquireError> = Ok(true);
        assert_eq!(classify_prompt_result(res, "confirm").unwrap(), Some(true));
    }

    #[test]
    fn classify_maps_both_cancel_variants_to_none() {
        for err in [
            inquire::InquireError::OperationCanceled,
            inquire::InquireError::OperationInterrupted,
        ] {
            let res: Result<String, _> = Err(err);
            assert_eq!(classify_prompt_result(res, "text").unwrap(), None);
        }
    }

    #[test]
    fn classify_propagates_other_variants_with_the_prompt_source_named() {
        let res: Result<Vec<String>, _> = Err(inquire::InquireError::NotTTY);
        let err = classify_prompt_result(res, "theme select").unwrap_err();
        assert!(
            format!("{err:#}").contains("theme select prompt failed"),
            "got: {err:#}"
        );
        assert!(cancellation_of(&err).is_none());
    }

    #[test]
    fn require_answer_turns_a_cancel_into_the_shared_sigint_error() {
        for err in [
            inquire::InquireError::OperationCanceled,
            inquire::InquireError::OperationInterrupted,
        ] {
            let res: Result<u8, _> = Err(err);
            let err = require_answer(res, "new-command").unwrap_err();
            assert_eq!(crate::extract_exit_code_override(&err), Some(SIGINT_EXIT));
            assert_eq!(
                cancellation_of(&err).map(ToString::to_string),
                Some("new-command cancelled".to_string())
            );
        }
    }

    #[test]
    fn require_answer_passes_a_real_answer_through() {
        let res: Result<u8, inquire::InquireError> = Ok(7);
        assert_eq!(require_answer(res, "new-command").unwrap(), 7);
    }
}
