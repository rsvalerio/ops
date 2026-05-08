//! Secret-pattern heuristics used to warn when sensitive values appear in
//! command env definitions, and to redact values in dry-run output.
//!
//! These detectors are intentionally conservative: they err toward not
//! flagging rather than risking false-positive noise on build-config
//! identifiers like `version_1_2_3`. See `SEC-002` for the surrounding
//! threat model documented in `command::exec`.

/// DUP-001 / DUP-3: Patterns that both warn and redact in dry-run output.
/// Single source of truth; the warn list is `SENSITIVE_REDACTION_PATTERNS`
/// chained with [`WARN_ONLY_PATTERNS`].
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

/// Patterns that only trigger a warning. Kept separate because they commonly
/// appear in non-secret contexts (e.g. `AWS_ACCESS_KEY_ID` is half a credential
/// pair but not itself confidential, `SESSION_*` often refers to UI state).
const WARN_ONLY_PATTERNS: &[&str] = &["access_key", "session"];

fn warn_patterns() -> impl Iterator<Item = &'static &'static str> {
    SENSITIVE_REDACTION_PATTERNS
        .iter()
        .chain(WARN_ONLY_PATTERNS.iter())
}

/// SEC-002: Warn if environment variable key or value looks sensitive.
///
/// Checks for:
/// - Key names containing patterns from `SENSITIVE_KEY_PATTERNS`
/// - Values that look like secrets: long base64-like strings, AWS-style keys,
///   JWT-like tokens
pub fn warn_if_sensitive_env(key: &str, value: &str) {
    let key_bytes = key.as_bytes();
    for pattern in warn_patterns() {
        if ascii_contains_ignore_case(key_bytes, pattern.as_bytes()) {
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
    let key_bytes = key.as_bytes();
    SENSITIVE_REDACTION_PATTERNS
        .iter()
        .any(|p| ascii_contains_ignore_case(key_bytes, p.as_bytes()))
}

/// PERF-3 (TASK-1053): allocation-free ASCII case-insensitive substring search.
///
/// Walks `haystack.windows(needle.len())` and compares byte-by-byte with
/// [`u8::eq_ignore_ascii_case`]. Patterns are pure ASCII by construction
/// (see [`SENSITIVE_REDACTION_PATTERNS`] / [`WARN_ONLY_PATTERNS`]), and env
/// keys are virtually always ASCII; non-ASCII bytes in the haystack simply
/// fail the byte comparison and the window slides forward, preserving the
/// previous `key.to_lowercase().contains(pattern)` semantics for ASCII input.
fn ascii_contains_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if haystack.len() < needle.len() {
        return false;
    }
    haystack.windows(needle.len()).any(|window| {
        window
            .iter()
            .zip(needle)
            .all(|(h, n)| h.eq_ignore_ascii_case(n))
    })
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

    /// Redaction is a strict subset of warn by construction: warn iterates
    /// `SENSITIVE_REDACTION_PATTERNS.chain(WARN_ONLY_PATTERNS)`. This test is
    /// trivially true and exists to flag a future regression where someone
    /// breaks the chained construction.
    #[test]
    fn redaction_patterns_is_subset_of_warn_patterns() {
        let warn: Vec<&&str> = warn_patterns().collect();
        for pattern in SENSITIVE_REDACTION_PATTERNS {
            assert!(warn.iter().any(|p| **p == *pattern));
        }
    }

    /// SEC-16/TEST-15 (TASK-1098): scanning a multi-megabyte value must not
    /// look beyond `SECRET_SCAN_LIMIT`. Previous wall-clock `< 1s` assertion
    /// was flaky on shared/sanitised CI runners. Replaced with a behavioural
    /// proxy: build a value whose first `SECRET_SCAN_LIMIT` bytes are a
    /// uniform lowercase run (no digits/uppercase, so `has_high_entropy` is
    /// false) and whose tail is high-entropy enough that, if scanned, would
    /// flip the prefix-friendly detectors to `true`. The length-pinned
    /// detectors (`looks_like_aws_key`, `looks_like_uuid`) reject the value
    /// outright on length grounds, so the result is fully determined by
    /// whether the scan honoured the cap. Identical-to-truncated behaviour
    /// proves no byte past the cap was consulted.
    #[test]
    fn looks_like_secret_value_does_not_scan_past_cap() {
        let prefix = "a".repeat(SECRET_SCAN_LIMIT);
        // High-entropy tail: enough digits, lowercase, uppercase to easily
        // satisfy `has_high_entropy` if the scan looked at it. Sized to push
        // total length well past 2 MiB.
        let tail_unit = "Aa1Bb2Cc3Dd4Ee5Ff6Gg7Hh8Ii9Jj0";
        let tail = tail_unit.repeat((2 * 1024 * 1024) / tail_unit.len() + 1);
        let huge = format!("{prefix}{tail}");
        assert!(huge.len() > 2 * 1024 * 1024);

        // Sanity: the high-entropy tail in isolation would flag.
        assert!(
            looks_like_secret_value(&tail),
            "test fixture is wrong: tail must trip has_high_entropy"
        );

        // The capped scan must agree with what it sees on the truncated
        // prefix alone — proving bytes past SECRET_SCAN_LIMIT were not read.
        assert_eq!(
            looks_like_secret_value(&huge),
            looks_like_secret_value(&prefix),
            "scan consulted bytes past SECRET_SCAN_LIMIT ({SECRET_SCAN_LIMIT})"
        );
        assert!(
            !looks_like_secret_value(&huge),
            "uniform-prefix value within cap should not look like a secret"
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

    /// PERF-3 (TASK-1053): the allocation-free ASCII case-insensitive
    /// substring matcher must agree with the previous
    /// `key.to_lowercase().contains(pattern)` behavior across upper/lower/
    /// mixed case input for both warn and redaction decisions.
    #[test]
    fn sensitive_key_detection_parity_across_case() {
        // Reference oracle: the original allocating implementation.
        fn warn_oracle(key: &str) -> bool {
            let lower = key.to_lowercase();
            SENSITIVE_REDACTION_PATTERNS
                .iter()
                .chain(WARN_ONLY_PATTERNS.iter())
                .any(|p| lower.contains(p))
        }
        fn redact_oracle(key: &str) -> bool {
            let lower = key.to_lowercase();
            SENSITIVE_REDACTION_PATTERNS
                .iter()
                .any(|p| lower.contains(p))
        }

        // Sensitive (warn): should match patterns from either list.
        // Non-sensitive: should not match any pattern.
        // Mix cases to confirm the case-fold is correct.
        let cases = [
            "aws_secret_access_key",
            "AWS_SECRET_ACCESS_KEY",
            "Aws_Secret_Access_Key",
            "MY_API_KEY",
            "github_token",
            "DB_PASSWORD",
            "X_PRIVATE_VALUE",
            "auth_header",
            "AWS_ACCESS_KEY_ID", // warn-only (access_key)
            "session_id",        // warn-only
            "PATH",
            "HOME",
            "CARGO_HOME",
            "RUST_LOG",
            "version_1_2_3",
            "",
            "tok", // shorter than any pattern
        ];

        for key in cases {
            // Redaction parity:
            assert_eq!(
                is_sensitive_env_key(key),
                redact_oracle(key),
                "redaction mismatch for {key:?}"
            );

            // Warn-decision parity: replicate the warn-key branch in isolation.
            let key_bytes = key.as_bytes();
            let warn_decision =
                warn_patterns().any(|p| ascii_contains_ignore_case(key_bytes, p.as_bytes()));
            assert_eq!(warn_decision, warn_oracle(key), "warn mismatch for {key:?}");
        }

        // Spot-check explicit expectations called out in TASK-1053.
        assert!(is_sensitive_env_key("aws_secret_access_key"));
        assert!(!is_sensitive_env_key("PATH"));
    }

    #[test]
    fn ascii_contains_ignore_case_basics() {
        assert!(ascii_contains_ignore_case(b"AWS_SECRET", b"secret"));
        assert!(ascii_contains_ignore_case(b"hello_TOKEN_x", b"token"));
        assert!(ascii_contains_ignore_case(b"abc", b""));
        assert!(!ascii_contains_ignore_case(b"PATH", b"secret"));
        assert!(!ascii_contains_ignore_case(b"ab", b"abc"));
        // Non-ASCII bytes do not match ASCII patterns (consistent with the
        // previous to_lowercase-then-contains for ASCII patterns: a non-ASCII
        // byte cannot equal an ASCII pattern byte under ASCII case fold).
        let non_ascii = "héllo_secret".as_bytes();
        assert!(ascii_contains_ignore_case(non_ascii, b"secret"));
    }

    #[test]
    fn bounded_prefix_respects_char_boundaries() {
        let s = "ééééé"; // each é is 2 bytes
        let p = bounded_prefix(s, 5);
        assert!(s.starts_with(p));
        assert_eq!(p.len(), 4);
    }
}
