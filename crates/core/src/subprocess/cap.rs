//! ARCH-1 / TASK-1471: env-knob parsers extracted from
//! [`super`][`crate::subprocess`].
//!
//! Houses the timeout and per-stream byte-cap env knobs so the parsing /
//! clamping / one-shot-warn semantics stay co-located, separate from the
//! drain machinery (`drain.rs`) and the public `run_with_timeout` shell
//! (`mod.rs`).

use std::sync::OnceLock;
use std::time::Duration;

/// Environment variable used to override the per-operation default timeout.
pub const TIMEOUT_ENV: &str = "OPS_SUBPROCESS_TIMEOUT_SECS";

/// SEC-33 / TASK-1050: environment variable used to override the per-stream
/// byte cap applied by [`super::run_with_timeout`]'s drain threads. Mirrors
/// the runner's `command::exec::read_capped` shape (PERF-1 / TASK-0764) so a
/// runaway cargo subprocess cannot grow the in-memory capture buffer
/// without bound. Reuses the same env var name the runner already
/// documents — `ops` users only have one knob to tune.
pub const OUTPUT_CAP_ENV: &str = "OPS_OUTPUT_BYTE_CAP";

/// Default per-stream byte cap applied to captured stdout/stderr in
/// [`super::run_with_timeout`]. Matches the runner's
/// `DEFAULT_OUTPUT_BYTE_CAP` (4 MiB) so the cap is consistent across the
/// project's two subprocess paths. Once the cap is reached the drain
/// thread keeps reading from the pipe (so the child does not block on a
/// full pipe and risk a timeout) but discards the bytes and increments a
/// `dropped` counter that surfaces via `tracing::warn!`.
pub const DEFAULT_OUTPUT_BYTE_CAP: usize = 4 * 1024 * 1024;

/// Fallback timeout applied when a caller has no operation-specific default
/// and `OPS_SUBPROCESS_TIMEOUT_SECS` is unset or unparseable.
pub const FALLBACK_TIMEOUT: Duration = Duration::from_secs(180);

/// ASYNC-6 / TASK-0304: upper bound on `OPS_SUBPROCESS_TIMEOUT_SECS`.
///
/// The whole point of [`super::run_with_timeout`] is bounded execution;
/// allowing an env-driven `u64::MAX` effectively disables the timeout and
/// silently breaks the helper's contract. 1 hour is generous (the longest
/// legitimate caller is `cargo update`, capped well below this) while still
/// preventing an unbounded hang.
pub const MAX_TIMEOUT_SECS: u64 = 3600;

/// SEC-33 / TASK-1050 + ARCH-11 / TASK-1463: resolve the per-stream byte cap
/// once per process. Routes through the shared
/// [`crate::text::cached_byte_cap_env`] helper so the unset / zero /
/// unparseable / `> BYTE_CAP_ENV_MAX` matrix and the one-shot
/// `tracing::warn!` discipline stay aligned with `manifest_max_bytes` and
/// `ops_toml_max_bytes`. Without the shared helper an
/// `OPS_OUTPUT_BYTE_CAP=18446744073709551615` silently disabled the SEC-33
/// cap for subprocess capture — exactly the failure mode the upper-bound
/// clamp was added to prevent.
pub(super) fn output_byte_cap() -> usize {
    static CAP: OnceLock<u64> = OnceLock::new();
    let resolved =
        crate::text::cached_byte_cap_env(&CAP, OUTPUT_CAP_ENV, DEFAULT_OUTPUT_BYTE_CAP as u64);
    // `cached_byte_cap_env` clamps to `BYTE_CAP_ENV_MAX` (1 GiB) which fits
    // in `usize` on every target we build for (32-bit and 64-bit), so the
    // `try_from` is total. The fallback is defence-in-depth for a
    // hypothetical 16-bit target.
    usize::try_from(resolved).unwrap_or(DEFAULT_OUTPUT_BYTE_CAP)
}

/// PERF-3 / TASK-1218: pure parser for the `OPS_SUBPROCESS_TIMEOUT_SECS`
/// raw value. Returns `Some(secs)` (clamped to [`MAX_TIMEOUT_SECS`]) when
/// the input is a positive `u64`, `None` otherwise so the caller falls
/// back to the operation-specific default. Factored out so the
/// clamp/zero/unset matrix is unit-testable without poking the
/// process-global `OnceLock`.
pub(super) fn parse_subprocess_timeout(raw: Option<&str>) -> Option<u64> {
    let parsed = raw?.parse::<u64>().ok().filter(|&s| s > 0)?;
    if parsed > MAX_TIMEOUT_SECS {
        tracing::warn!(
            requested = parsed,
            clamped_to = MAX_TIMEOUT_SECS,
            env = TIMEOUT_ENV,
            "ASYNC-6: clamping subprocess timeout to upper bound; bounded execution is the helper's contract"
        );
        Some(MAX_TIMEOUT_SECS)
    } else {
        Some(parsed)
    }
}

/// PERF-3 / TASK-1218: cache the resolved `OPS_SUBPROCESS_TIMEOUT_SECS`
/// value behind a `OnceLock<Option<u64>>` so each subprocess spawn does
/// not re-acquire the global env lock and re-allocate the raw `String`.
/// `None` means "env unset / zero / unparseable — fall back to
/// op_default"; `Some(secs)` is the already-clamped override (the warn
/// fires once at cache init). Mirrors the `output_byte_cap` discipline
/// one function above.
fn cached_subprocess_timeout() -> Option<u64> {
    static CACHED: OnceLock<Option<u64>> = OnceLock::new();
    *CACHED.get_or_init(|| parse_subprocess_timeout(std::env::var(TIMEOUT_ENV).ok().as_deref()))
}

/// Resolve an effective timeout: `OPS_SUBPROCESS_TIMEOUT_SECS` overrides the
/// caller-provided default if present and parses to a non-zero u64; otherwise
/// the operation-specific default is returned unchanged.
///
/// ASYNC-6 / TASK-0304: the override is clamped to [`MAX_TIMEOUT_SECS`] and
/// emits a warning when it had to be clamped, so an accidental
/// `OPS_SUBPROCESS_TIMEOUT_SECS=18446744073709551615` does not silently
/// disable the helper's bounded-wait contract.
///
/// PERF-3 / TASK-1218: the env knob is resolved at most once per process
/// via [`cached_subprocess_timeout`]. Tests that exercise the parse/clamp
/// matrix should call [`parse_subprocess_timeout`] directly to bypass the
/// cache.
#[must_use]
pub fn default_timeout(op_default: Duration) -> Duration {
    match cached_subprocess_timeout() {
        Some(secs) => Duration::from_secs(secs),
        None => op_default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ASYNC-6 / TASK-0304 + PERF-3 / TASK-1218: the matrix tests now
    /// exercise the pure [`parse_subprocess_timeout`] helper directly. The
    /// public [`default_timeout`] memoises via `cached_subprocess_timeout`,
    /// so a serial-mutating test would either be the first to populate the
    /// cache (winning) or observe a stale snapshot, depending on test
    /// ordering — exactly the surprise the cache is meant to remove.
    mod env_override {
        use super::*;

        #[test]
        fn clamps_huge_value_to_max() {
            let secs = parse_subprocess_timeout(Some(&u64::MAX.to_string()))
                .expect("huge value clamps but is still Some");
            assert_eq!(secs, MAX_TIMEOUT_SECS);
        }

        #[test]
        fn zero_value_falls_back_to_op_default() {
            assert!(
                parse_subprocess_timeout(Some("0")).is_none(),
                "zero must fall back so default_timeout returns op_default"
            );
        }

        #[test]
        fn unset_returns_op_default() {
            assert!(
                parse_subprocess_timeout(None).is_none(),
                "unset must fall back so default_timeout returns op_default"
            );
        }

        #[test]
        fn within_bounds_is_honored() {
            assert_eq!(parse_subprocess_timeout(Some("30")), Some(30));
        }

        #[test]
        fn unparseable_falls_back() {
            assert!(parse_subprocess_timeout(Some("not-a-number")).is_none());
        }
    }

    /// ARCH-11 / TASK-1463: `output_byte_cap` must route through the shared
    /// `cached_byte_cap_env` parser so an
    /// `OPS_OUTPUT_BYTE_CAP=18446744073709551615` clamps to
    /// `text::BYTE_CAP_ENV_MAX` (1 GiB) rather than silently disabling the
    /// SEC-33 cap for subprocess capture. The production `OnceLock` is
    /// process-global, so the test drives a fresh local slot through the
    /// shared parser to pin the clamp without poisoning the production
    /// cache.
    #[test]
    #[serial_test::serial]
    fn output_byte_cap_clamps_u64_max_to_env_max() {
        let prev = std::env::var_os(OUTPUT_CAP_ENV);
        // SAFETY: `#[serial]` ensures no parallel test observes this env
        // mutation window. Restored before any assertion.
        unsafe { std::env::set_var(OUTPUT_CAP_ENV, u64::MAX.to_string()) };
        static SLOT: OnceLock<u64> = OnceLock::new();
        let resolved =
            crate::text::cached_byte_cap_env(&SLOT, OUTPUT_CAP_ENV, DEFAULT_OUTPUT_BYTE_CAP as u64);
        match prev {
            Some(v) => unsafe { std::env::set_var(OUTPUT_CAP_ENV, v) },
            None => unsafe { std::env::remove_var(OUTPUT_CAP_ENV) },
        }
        assert_eq!(
            resolved,
            crate::text::BYTE_CAP_ENV_MAX,
            "u64::MAX must clamp to the shared BYTE_CAP_ENV_MAX, got {resolved}"
        );
    }
}
