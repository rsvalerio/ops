//! ARCH-1 / TASK-1471: `OPS__` environment-variable overlay merge.
//!
//! Extracted from the historical grab-bag `loader.rs` so the env-overlay
//! concerns (the `config` crate's prefix source, the non-UTF-8 OPS__ key
//! diagnostic, and the success-path early-out) live alongside their own
//! tests.

use anyhow::Context;
use config as config_crate;

use super::super::{merge::merge_config, Config, ConfigOverlay};

/// Merge environment variables with OPS prefix into config.
///
/// Only applies overlay when OPS__ prefixed env vars exist.
/// Without this guard, the `config` crate deserializes an empty config with
/// all-default values, and merge_config unconditionally overwrites the local
/// config's intentional settings.
///
/// Fails fast on deserialization errors (SEC-11 / ERR-1): a mistyped
/// `OPS__OUTOUT__THEME` in CI should surface as a loud error rather than a
/// silent misconfiguration that drops every other OPS__ variable.
///
/// ERR-1 / TASK-1389: a non-UTF-8 `OPS__*` key (rare but possible on Unix —
/// e.g. an exec'd shim that wrote raw bytes via `OsString::from_vec`) is
/// invisible to the `config` crate's `Environment::with_prefix("OPS")` source
/// but is still operator intent. Count and `tracing::warn!` once per
/// `merge_env_vars` call so the "OPS__ override didn't apply" symptom has a
/// breadcrumb instead of vanishing silently.
///
/// PERF-3 / TASK-1414: the success-path early-out (`vars_os().any(...)`)
/// short-circuits without allocating a `Vec<String>` of every OPS__ key. The
/// error-context closures are the only callers that need the materialised key
/// list, so the collection is deferred into [`collect_ops_keys`] and only runs
/// on the failure path.
pub(super) fn merge_env_vars(config: &mut Config) -> anyhow::Result<()> {
    let (has_ops_keys, non_utf8_count) = scan_ops_env_keys();
    if non_utf8_count > 0 {
        tracing::warn!(
            count = non_utf8_count,
            "ignored non-UTF-8 OPS__ environment keys; the `config` crate cannot \
             observe them — operator overrides relying on these keys will not apply"
        );
    }
    if !has_ops_keys {
        return Ok(());
    }
    let env_config = config_crate::Config::builder()
        .add_source(config_crate::Environment::with_prefix("OPS").separator("__"))
        .build()
        .with_context(|| {
            let keys = collect_ops_keys();
            format!("failed to build OPS__ env config (keys: {keys:?})")
        })?;
    let env_overlay: ConfigOverlay = env_config.try_deserialize().with_context(|| {
        let keys = collect_ops_keys();
        format!("failed to deserialize OPS__ env config (keys: {keys:?})")
    })?;
    merge_config(config, env_overlay);
    Ok(())
}

/// Return `(has_ops_keys, non_utf8_count)` for the current process env.
///
/// PERF-3 / TASK-1414: avoids the `Vec<String>` allocation on the success
/// path. ERR-1 / TASK-1389: tracks non-UTF-8 `OPS__*` keys via the raw
/// `OsStr::as_encoded_bytes` prefix so the diagnostic warn can fire even when
/// `OsString::into_string()` would have dropped the entry.
fn scan_ops_env_keys() -> (bool, usize) {
    let mut has_ops = false;
    let mut non_utf8 = 0usize;
    for (k, _) in std::env::vars_os() {
        match k.to_str() {
            Some(s) if s.starts_with("OPS__") => has_ops = true,
            Some(_) => {}
            None => {
                if k.as_encoded_bytes().starts_with(b"OPS__") {
                    non_utf8 += 1;
                }
            }
        }
    }
    (has_ops, non_utf8)
}

/// Collect the UTF-8 `OPS__*` env keys. Only the error-context closures call
/// this; the success path skips it entirely (TASK-1414).
fn collect_ops_keys() -> Vec<String> {
    std::env::vars_os()
        .filter_map(|(k, _)| k.into_string().ok())
        .filter(|k| k.starts_with("OPS__"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ERR-1 / TASK-1389: a non-UTF-8 `OPS__*` key in the process env is
    /// invisible to the `config` crate's `Environment::with_prefix("OPS")`
    /// source. `scan_ops_env_keys` must surface the count so
    /// [`merge_env_vars`] can emit the diagnostic warn (rather than dropping
    /// it silently as the prior `into_string().ok()` filter did).
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn scan_ops_env_keys_counts_non_utf8_ops_keys() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        // Build a key that begins with `OPS__` but contains non-UTF-8 trailing
        // bytes. The raw bytes are valid as an OsString but `into_string()`
        // returns Err so the previous diagnostic path would have dropped it.
        let mut raw = b"OPS__BAD_".to_vec();
        raw.extend_from_slice(&[0xff, 0xfe, 0xfd]);
        let key: OsString = OsString::from_vec(raw);
        // SAFETY: test-only guard via #[serial] attribute.
        unsafe { std::env::set_var(&key, "x") };

        let (_, non_utf8) = scan_ops_env_keys();

        // SAFETY: test-only guard via #[serial] attribute.
        unsafe { std::env::remove_var(&key) };

        assert!(non_utf8 >= 1, "non-UTF-8 OPS__ key must be counted");
    }

    /// ERR-1 / TASK-1389: with no non-UTF-8 OPS__ keys present, the diagnostic
    /// counter must stay at zero so the warn does not fire spuriously.
    #[test]
    #[serial_test::serial]
    fn scan_ops_env_keys_zero_when_only_utf8_keys() {
        // The harness env may already carry OPS__ vars from prior tests; this
        // assertion only pins the non-UTF-8 counter, not the presence flag.
        let (_, non_utf8) = scan_ops_env_keys();
        assert_eq!(
            non_utf8, 0,
            "no non-UTF-8 OPS__ keys expected in baseline env"
        );
    }
}
