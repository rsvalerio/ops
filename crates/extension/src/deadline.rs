//! The provider dispatch budget: [`Deadline`] and
//! [`DEFAULT_PROVIDER_BUDGET`].
//!
//! ARCH-1 / TASK-2095: split out of `data.rs` — the budget type and its
//! config resolution form a cohesive unit distinct from both the registry
//! surface (`crate::data`) and the per-invocation state
//! ([`crate::context::Context`]) that installs it.

use crate::error::DataProviderError;
use ops_core::config::Config;
use std::time::{Duration, Instant};

/// SEC-33 / TASK-2017: default wall-clock budget for one provider dispatch.
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
/// CONC-9 / TASK-2068: this used to carry the ordering requirement as prose —
/// the budget had to stay **above every subprocess timeout a provider can wait
/// on**, or it would fire first and report a run still within its own limit as
/// a failure. TASK-2056 made the budget operator-configurable
/// (`[data] provider_budget_secs`), which put that invariant at the mercy of a
/// config file no test could police. The binding subprocess wait,
/// `ops-test-coverage`'s `CARGO_LLVM_COV_TIMEOUT` (15 minutes), now sizes
/// itself from [`crate::context::Context::deadline`] instead, so the two agree by construction
/// at whatever value this budget takes and the ordering no longer has to be
/// maintained by hand.
///
/// The twenty minutes still buy headroom over that fifteen for the workspace
/// walk and parsing either side of the subprocess. A provider that hands work
/// to something with its own timeout knob should follow the coverage provider
/// and size it from [`crate::context::Context::deadline`] rather than assume a floor here.
pub const DEFAULT_PROVIDER_BUDGET: Duration = Duration::from_mins(20);

/// The budget installed for the provider dispatch currently in flight.
///
/// Held by [`crate::context::Context`] for the duration of the outermost
/// [`crate::data::DataRegistry::provide`] call and inherited by every provider that one
/// composes, so a provider graph cannot multiply its budget by nesting.
///
/// SEC-33 / TASK-2052: public and detachable ([`crate::context::Context::deadline_handle`])
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
    /// SEC-33 / TASK-2017: construct from parts. `pub(crate)` — the fields
    /// stay private to this module (ARCH-1 / TASK-2095 split) so external
    /// code cannot forge an already-expired or mis-attributed budget.
    pub(crate) const fn from_parts(
        provider: String,
        budget: Duration,
        expires_at: Instant,
    ) -> Self {
        Self {
            provider,
            budget,
            expires_at,
        }
    }

    /// When this dispatch's budget runs out.
    #[must_use]
    pub const fn expires_at(&self) -> Instant {
        self.expires_at
    }

    /// Whether the budget has already run out.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// The cooperative cancellation point, for code that holds a detached
    /// deadline rather than a `&Context`.
    ///
    /// # Errors
    ///
    /// [`DataProviderError::TimedOut`], naming the provider that owns the
    /// budget, once the deadline has passed.
    pub fn check(&self) -> Result<(), DataProviderError> {
        if self.is_expired() {
            return Err(DataProviderError::TimedOut {
                provider: self.provider.clone(),
                budget: self.budget,
            });
        }
        Ok(())
    }
}

/// CONC-9 / TASK-2056: resolve the dispatch budget an operator configured,
/// falling back to [`DEFAULT_PROVIDER_BUDGET`].
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
