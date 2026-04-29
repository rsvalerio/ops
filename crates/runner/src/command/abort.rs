//! CONC-9 / TASK-0571: cooperative abort signal for parallel exec.
//!
//! Replaces the prior `Arc<AtomicBool>` + `yield_now` busy-poll loops in
//! `exec_standalone`. Under `MAX_PARALLEL = 32` there were up to 64 such
//! loops live simultaneously, each waking the executor on every poll
//! cycle, burning CPU and inflating wakeup latency for real I/O.
//!
//! `AbortSignal` pairs an `AtomicBool` (so synchronous `is_set` checks at
//! task entry stay cheap and lock-free) with a `tokio::sync::Notify` so
//! awaiters block until the flag flips instead of spin-yielding.
//!
//! Use `cancelled()` in `tokio::select!` arms to race against the abort
//! signal:
//!
//! ```text
//! tokio::select! {
//!     biased;
//!     _ = some_io_op() => { ... }
//!     () = abort.cancelled() => { /* fail_fast tripped */ }
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

/// Cooperative cancellation signal shared across parallel tasks.
///
/// Cheap to clone via `Arc<AbortSignal>`. `set()` is idempotent and
/// awakens all current and future awaiters of `cancelled()`.
#[derive(Debug, Default)]
pub(crate) struct AbortSignal {
    flag: AtomicBool,
    notify: Notify,
}

impl AbortSignal {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Synchronous, lock-free check. Cheap enough to use on hot paths
    /// (e.g. the entry-of-task check in `exec_standalone`).
    pub(crate) fn is_set(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Trip the abort and wake every current and future `cancelled()`
    /// awaiter. Idempotent — calling `set` more than once has no
    /// additional effect.
    pub(crate) fn set(&self) {
        // `swap` so we only emit a notify once, even under concurrent set
        // calls — saves a redundant wake-burst when fail_fast and a task
        // shutdown race.
        if !self.flag.swap(true, Ordering::AcqRel) {
            self.notify.notify_waiters();
        }
    }

    /// Future that resolves once `set()` has been called. Returns
    /// immediately if the signal was already tripped before the call.
    ///
    /// The double-check around `Notified` closes the lost-wakeup race:
    /// once we register interest via `notified()` we re-read the flag,
    /// so a `set()` that lands between the first load and the
    /// registration is still observed.
    pub(crate) async fn cancelled(&self) {
        if self.is_set() {
            return;
        }
        loop {
            let notified = self.notify.notified();
            if self.is_set() {
                return;
            }
            notified.await;
            if self.is_set() {
                return;
            }
            // Spurious wake (notify_waiters fired but flag not set yet).
            // Re-arm and re-check.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn cancelled_resolves_after_set() {
        let signal = Arc::new(AbortSignal::new());
        assert!(!signal.is_set());

        let waiter = {
            let s = Arc::clone(&signal);
            tokio::spawn(async move { s.cancelled().await })
        };
        // Yield so the waiter actually parks on `notified().await`.
        tokio::task::yield_now().await;

        signal.set();
        // bounded wait so a regression doesn't hang CI
        tokio::time::timeout(Duration::from_secs(1), waiter)
            .await
            .expect("waiter must wake within 1s after set()")
            .expect("waiter task ok");
        assert!(signal.is_set());
    }

    #[tokio::test]
    async fn cancelled_returns_immediately_when_already_set() {
        let signal = AbortSignal::new();
        signal.set();
        // Should resolve without parking on Notify.
        tokio::time::timeout(Duration::from_millis(10), signal.cancelled())
            .await
            .expect("already-set signal must resolve immediately");
    }

    #[tokio::test]
    async fn set_is_idempotent() {
        let signal = AbortSignal::new();
        signal.set();
        signal.set();
        assert!(signal.is_set());
    }

    /// CONC-9 regression: many waiters under MAX_PARALLEL must all wake
    /// from a single set, with no busy-polling.
    #[tokio::test]
    async fn many_waiters_all_wake_on_set() {
        let signal = Arc::new(AbortSignal::new());
        let mut handles = Vec::new();
        for _ in 0..64 {
            let s = Arc::clone(&signal);
            handles.push(tokio::spawn(async move { s.cancelled().await }));
        }
        tokio::task::yield_now().await;
        signal.set();
        for h in handles {
            tokio::time::timeout(Duration::from_secs(1), h)
                .await
                .expect("waiter wake within 1s")
                .expect("ok");
        }
    }
}
