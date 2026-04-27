//! Secret-pattern heuristics used to warn when sensitive values appear in
//! command env definitions, and to redact values in dry-run output.
//!
//! These detectors are intentionally conservative: they err toward not
//! flagging rather than risking false-positive noise on build-config
//! identifiers like `version_1_2_3`. See `SEC-002` for the surrounding
//! threat model documented in `command::exec`.

/// DUP-001: Shared patterns for detecting sensitive environment variable names.
/// Used by `warn_if_sensitive_env` for warnings and `is_sensitive_env_key` for
/// dry-run redaction.
///
/// `SENSITIVE_REDACTION_PATTERNS` is a strict subset of this list. The extra
/// entries ("access_key", "session") trigger warnings but are not redacted in
/// dry-run output because they commonly appear in non-secret contexts.
const SENSITIVE_KEY_PATTERNS: &[&str] = &[
    "password",
    "secret",
    "token",
    "api_key",
    "apikey",
    "private",
    "credential",
    "auth",
    "access_key",
    "session",
];

/// DUP-001: Subset of `SENSITIVE_KEY_PATTERNS` used for dry-run redaction.
/// Every entry here must also appear in `SENSITIVE_KEY_PATTERNS`.
const SENSITIVE_REDACTION_PATTERNS: &[&str] = &[
    "password",
    "secret",
    "token",
    "api_key",
    "apikey",
    "private",
    "credential",
    "auth",
];

/// SEC-002: Warn if environment variable key or value looks sensitive.
///
/// Checks for:
/// - Key names containing patterns from `SENSITIVE_KEY_PATTERNS`
/// - Values that look like secrets: long base64-like strings, AWS-style keys,
///   JWT-like tokens
pub fn warn_if_sensitive_env(key: &str, value: &str) {
    let lower = key.to_lowercase();
    for pattern in SENSITIVE_KEY_PATTERNS {
        if lower.contains(pattern) {
            tracing::warn!(
                key = %key,
                "SEC-002: env variable name suggests sensitive data; use OS environment instead of config"
            );
            return;
        }
    }

    if looks_like_secret_value(value) {
        tracing::warn!(
            key = %key,
            value_len = value.len(),
            "SEC-002: env variable value looks like a secret (long random-looking string); use OS environment instead of config"
        );
    }
}

/// DUP-001: Check if an env key looks like it might contain sensitive data.
///
/// This is used by dry-run mode to redact sensitive values in output.
/// Returns true if the key name suggests it contains a secret.
pub fn is_sensitive_env_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    SENSITIVE_REDACTION_PATTERNS
        .iter()
        .any(|p| lower.contains(p))
}

/// SEC-16: Upper bound on bytes scanned by [`looks_like_secret_value`].
///
/// Every command spawn calls this on the value of every configured env var.
/// An accidentally huge value (multi-MB blob piped in via config) would turn
/// the detector into an O(n) hot-path bottleneck. 4 KiB comfortably covers
/// every credential format we care about (AWS keys are 40B, JWTs typically
/// well under 2 KiB, UUIDs 36B) while bounding worst-case work per spawn.
const SECRET_SCAN_LIMIT: usize = 4096;

/// Check if a value looks like it might be a secret.
///
/// CQ-011: Uses named predicates for each detection strategy, making the
/// logic explicit and testable. Each predicate checks a specific pattern:
///
/// - `has_high_entropy`: Mixed alphanumeric with digits, lowercase, uppercase
/// - `looks_like_jwt`: Starts with "eyJ" (base64-encoded JSON) and contains "."
/// - `looks_like_aws_key`: 40 chars, alphanumeric plus +/=
/// - `looks_like_uuid`: 36 chars with 4 hyphens in UUID format
///
/// SEC-16: Scanning is bounded to the first [`SECRET_SCAN_LIMIT`] bytes so an
/// oversized env value cannot turn this detector into a per-spawn DoS.
pub fn looks_like_secret_value(value: &str) -> bool {
    if value.len() < 20 {
        return false;
    }

    let scan = bounded_prefix(value, SECRET_SCAN_LIMIT);

    // Length-pinned detectors (aws key, uuid) only match when the *full*
    // value matches their exact length, so they are still evaluated against
    // the original; truncating would let a huge value spuriously match by
    // accident or, conversely, miss a 40-char key that fits well under the
    // limit. has_high_entropy / looks_like_jwt are prefix-friendly.
    has_high_entropy(scan)
        || looks_like_jwt(scan)
        || looks_like_aws_key(value)
        || looks_like_uuid(value)
}

/// Return the longest UTF-8-safe prefix of `value` not exceeding `limit`
/// bytes. Truncating at an arbitrary byte index would split a multibyte
/// codepoint, so we round down to a char boundary.
fn bounded_prefix(value: &str, limit: usize) -> &str {
    if value.len() <= limit {
        return value;
    }
    let mut end = limit;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

/// CQ-005: Extracted helper predicates for secret detection.
///
/// Thresholds below are heuristic caps: a string is flagged as "high-entropy"
/// when it is long enough (>15 alphanumerics) and mixes digits, lowercase, and
/// uppercase in non-trivial amounts (>3 of each). This is deliberately strict
/// — legitimate words hit one or two of these but rarely all four — and keeps
/// false positives low on identifiers like `version_1_2_3`.
const HIGH_ENTROPY_MIN_ALPHANUMERIC: usize = 15;
const HIGH_ENTROPY_MIN_DIGITS: usize = 3;
const HIGH_ENTROPY_MIN_LOWERCASE: usize = 3;
const HIGH_ENTROPY_MIN_UPPERCASE: usize = 3;

pub(crate) fn has_high_entropy(value: &str) -> bool {
    let (mut alphanumeric, mut digits, mut lowercase, mut uppercase) = (0usize, 0, 0, 0);
    for c in value.chars() {
        if c.is_ascii_digit() {
            digits += 1;
            alphanumeric += 1;
        } else if c.is_ascii_lowercase() {
            lowercase += 1;
            alphanumeric += 1;
        } else if c.is_ascii_uppercase() {
            uppercase += 1;
            alphanumeric += 1;
        } else if c.is_alphanumeric() {
            alphanumeric += 1;
        }
    }
    alphanumeric > HIGH_ENTROPY_MIN_ALPHANUMERIC
        && digits > HIGH_ENTROPY_MIN_DIGITS
        && lowercase > HIGH_ENTROPY_MIN_LOWERCASE
        && uppercase > HIGH_ENTROPY_MIN_UPPERCASE
}

pub(crate) fn looks_like_jwt(value: &str) -> bool {
    value.starts_with("eyJ") && value.contains('.')
}

pub(crate) fn looks_like_aws_key(value: &str) -> bool {
    if value.len() != 40 {
        return false;
    }
    // SEC-11: AWS secret access keys are base64-ish and mix uppercase, lowercase,
    // digits, and +/=. Plain 40-char hex strings (git commit SHAs, many CI build
    // tokens) used to false-positive here, training operators to ignore the
    // SEC-002 warning. Requiring at least one non-hex character rules out lowercase
    // hex SHAs without excluding genuine AWS-shaped secrets, which virtually
    // always contain uppercase letters or +/=.
    let mut has_non_hex = false;
    for c in value.chars() {
        if !(c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=') {
            return false;
        }
        if !c.is_ascii_hexdigit() {
            has_non_hex = true;
        }
    }
    has_non_hex
}

pub(crate) fn looks_like_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == 5
        && parts[0].len() == 8
        && parts[1].len() == 4
        && parts[2].len() == 4
        && parts[3].len() == 4
        && parts[4].len() == 12
        && parts
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_patterns_is_subset_of_key_patterns() {
        for pattern in SENSITIVE_REDACTION_PATTERNS {
            assert!(
                SENSITIVE_KEY_PATTERNS.contains(pattern),
                "SENSITIVE_REDACTION_PATTERNS entry {pattern:?} missing from SENSITIVE_KEY_PATTERNS"
            );
        }
    }

    /// SEC-16: scanning a multi-megabyte value must complete promptly.
    /// On any reasonable host the bounded prefix keeps this in microseconds;
    /// the 1 s budget is a generous safety net against future regressions
    /// (CI noise, sanitisers) — a truly unbounded scan would still trip it.
    #[test]
    fn looks_like_secret_value_is_bounded_for_huge_values() {
        let huge = "a".repeat(2 * 1024 * 1024);
        let start = std::time::Instant::now();
        let flagged = looks_like_secret_value(&huge);
        let elapsed = start.elapsed();
        assert!(!flagged, "uniform 'a' run should not look like a secret");
        assert!(
            elapsed < std::time::Duration::from_secs(1),
            "scan took {elapsed:?} on a {}-byte value; SECRET_SCAN_LIMIT bound likely broken",
            huge.len()
        );
    }

    /// SEC-11: a 40-char lowercase-hex git commit SHA must not look like an
    /// AWS secret access key. Otherwise every spawn that has the commit SHA
    /// in env (CI is full of these) emits a SEC-002 warning recommending the
    /// user move it to OS env, which is noise.
    #[test]
    fn git_sha_does_not_look_like_secret() {
        let sha = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(sha.len(), 40);
        assert!(!looks_like_aws_key(sha));
        assert!(!looks_like_secret_value(sha));
    }

    /// Regression: a real AWS-shaped secret access key (mixed case, +/=) still
    /// trips the detector after the SEC-11 hex-only carve-out.
    #[test]
    fn aws_shaped_secret_still_flagged() {
        let key = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        assert_eq!(key.len(), 40);
        assert!(looks_like_aws_key(key));
    }

    #[test]
    fn bounded_prefix_respects_char_boundaries() {
        let s = "ééééé"; // each é is 2 bytes
        let p = bounded_prefix(s, 5);
        assert!(s.starts_with(p));
        assert_eq!(p.len(), 4);
    }
}
