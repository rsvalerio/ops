//! The provider dispatch budget: [`Deadline`] and
//! [`DEFAULT_PROVIDER_BUDGET`].
//!
//! The budget type and its config resolution form a cohesive unit distinct
//! from both the registry surface (`crate::data`) and the per-invocation
//! state ([`crate::context::Context`]) that installs it.

use crate::error::DataProviderError;
use ops_core::config::Config;
use std::time::{Duration, Instant};

/// Default wall-clock budget for one provider dispatch.
///
/// [`crate::data::DataProvider::provide`] is synchronous, so this is a *cooperative*
/// bound, not a preemptive one: it is enforced by
/// [`crate::context::Context::check_deadline`] inside providers that poll it and, for
/// providers that do not, by [`crate::data::DataRegistry::provide`] refusing to return a
/// value produced after the deadline. It exists so that no provider dispatch
/// is unbounded by construction; it cannot interrupt a thread already blocked
/// in a syscall.
///
/// The value is deliberately generous. Providers here range from a
/// sub-millisecond `Cargo.toml` read to `cargo llvm-cov` over a whole
/// workspace, and a budget tight enough to be interesting for the first would
/// turn the second into a spurious failure. Twenty minutes is an *upper bound
/// on a stall*, not a latency target. Callers that know their own tolerance
/// narrow it with [`crate::context::Context::with_provider_budget`].
///
/// The budget must stay **above every subprocess timeout a provider can wait
/// on**, or it fires first and reports a run still within its own limit as a
/// failure. The binding subprocess wait, `ops-test-coverage`'s
/// `CARGO_LLVM_COV_TIMEOUT` (15 minutes), therefore sizes itself from
/// [`crate::context::Context::deadline`], so the two agree by construction at
/// whatever value this budget (or `[data] provider_budget_secs`) takes. The
/// twenty minutes buy headroom over that fifteen for the workspace walk and
/// parsing either side of the subprocess. A provider that hands work to
/// something with its own timeout knob should size it from
/// [`crate::context::Context::deadline`] rather than assume a floor here.
pub const DEFAULT_PROVIDER_BUDGET: Duration = Duration::from_mins(20);

/// The budget installed for the provider dispatch currently in flight.
///
/// Held by [`crate::context::Context`] for the duration of the outermost
/// [`crate::data::DataRegistry::provide`] call and inherited by every provider that one
/// composes, so a provider graph cannot multiply its budget by nesting.
///
/// Public and detachable ([`crate::context::Context::deadline_handle`])
/// because the providers that most need to poll it are tree walkers whose
/// walk lives in a free function — sometimes, as in `rust-loc`, one that runs
/// on worker threads that cannot borrow `&Context` at all. A `Deadline` is
/// `Clone + Send + Sync` and carries everything [`Deadline::check`] needs to
/// build the same error [`crate::context::Context::check_deadline`] would, so threading it
/// into a walker does not weaken the failure an operator sees.
#[derive(Debug, Clone)]
pub struct Deadline {
    /// The provider that owns the budget — the outermost one, which is the
    /// one an operator asked for and the one worth naming in the failure.
    provider: String,
    budget: Duration,
    expires_at: Instant,
}

impl Deadline {
    /// Construct from parts. `pub(crate)` — the fields stay private to
    /// this module so external code cannot forge an already-expired or
    /// mis-attributed budget.
    pub(crate) const fn from_parts(
        provider: String,
        budget: Duration,
        expires_at: Instant,
    ) -> Self {
        Self { provider, budget, expires_at }
    }

    /// When this dispatch's budget runs out.
    #[must_use]
    pub const fn expires_at(&self) -> Instant {
        self.expires_at
    }

    /// Whether the budget has already run out.
    ///
    /// Reads the clock once and delegates to [`Deadline::is_expired_at`].
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.is_expired_at(Instant::now())
    }

    /// Whether the budget has run out as of `now`.
    ///
    /// The deadline is expired from [`Deadline::expires_at`] onwards,
    /// inclusive.
    #[must_use]
    pub fn is_expired_at(&self, now: Instant) -> bool {
        now >= self.expires_at
    }

    /// The cooperative cancellation point, for code that holds a detached
    /// deadline rather than a `&Context`.
    ///
    /// # Errors
    ///
    /// [`DataProviderError::TimedOut`], naming the provider that owns the
    /// budget, once the deadline has passed.
    pub fn check(&self) -> Result<(), DataProviderError> {
        self.check_at(Instant::now())
    }

    /// [`Deadline::check`] evaluated as of `now` rather than the current
    /// clock reading.
    ///
    /// # Errors
    ///
    /// [`DataProviderError::TimedOut`], naming the provider that owns the
    /// budget, when `now` is at or past the deadline.
    pub fn check_at(&self, now: Instant) -> Result<(), DataProviderError> {
        if self.is_expired_at(now) {
            return Err(DataProviderError::TimedOut {
                provider: self.provider.clone(),
                budget: self.budget,
            });
        }
        Ok(())
    }
}

/// Resolve the dispatch budget an operator configured, falling back to
/// [`DEFAULT_PROVIDER_BUDGET`].
///
/// `[data] provider_budget_secs = 0` is the documented opt-out and maps to
/// `None` (unbounded), which is the one value that must not be confused with
/// "unset": a zero-length budget would time every dispatch out instantly, so
/// reading it literally would turn a knob meant to *remove* the bound into
/// one that makes every provider fail.
pub const fn configured_provider_budget(config: &Config) -> Option<Duration> {
    match config.data.provider_budget_secs {
        None => Some(DEFAULT_PROVIDER_BUDGET),
        Some(0) => None,
        Some(secs) => Some(Duration::from_secs(secs)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUDGET: Duration = Duration::from_millis(20);

    fn deadline_at(expires_at: Instant) -> Deadline {
        Deadline::from_parts("slow".to_string(), BUDGET, expires_at)
    }

    #[test]
    fn not_expired_just_before_the_deadline() {
        let expires_at = Instant::now() + BUDGET;
        let deadline = deadline_at(expires_at);
        let just_before = expires_at
            .checked_sub(Duration::from_nanos(1))
            .expect("expires_at is a budget past now");
        assert!(!deadline.is_expired_at(just_before));
        assert!(deadline.check_at(just_before).is_ok());
    }

    #[test]
    fn expired_exactly_at_the_deadline() {
        let expires_at = Instant::now() + BUDGET;
        let deadline = deadline_at(expires_at);
        assert!(deadline.is_expired_at(expires_at));
        match deadline.check_at(expires_at) {
            Err(DataProviderError::TimedOut { provider, budget }) => {
                assert_eq!(provider, "slow");
                assert_eq!(budget, BUDGET);
            }
            other => panic!("expected TimedOut, got {other:?}"),
        }
    }

    #[test]
    fn expired_after_the_deadline() {
        let expires_at = Instant::now() + BUDGET;
        let deadline = deadline_at(expires_at);
        let after = expires_at + Duration::from_nanos(1);
        assert!(deadline.is_expired_at(after));
        assert!(deadline.check_at(after).is_err());
    }
}
