//! Tests for sensitive env detection.

use crate::command::secret_patterns::{
    has_high_entropy, is_sensitive_env_key, looks_like_aws_key, looks_like_jwt,
    looks_like_secret_value, looks_like_uuid,
};

#[test]
fn has_high_entropy_detects_random_strings() {
    assert!(has_high_entropy("AbCdEfGh123456789XyZ"));
    assert!(has_high_entropy("aB1cD2eF3gH4iJ5kL6"));
}

#[test]
fn has_high_entropy_rejects_simple_strings() {
    assert!(!has_high_entropy("simple"));
    assert!(!has_high_entropy("aaaaaaaaaaaa"));
    assert!(!has_high_entropy("12345678901234567890"));
}

#[test]
fn looks_like_jwt_detects_jwt_format() {
    assert!(looks_like_jwt(
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.signature"
    ));
    assert!(looks_like_jwt(
        "eyJzdWIiOiIxMjM0In0.eyJuYW1lIjoiSm9obiJ9.sig"
    ));
}

#[test]
fn looks_like_jwt_rejects_non_jwt() {
    assert!(!looks_like_jwt("not-a-jwt"));
    assert!(!looks_like_jwt("bearer eyJhbGciOiJIUzI1NiJ9"));
}

#[test]
fn looks_like_aws_key_detects_40_char_keys() {
    assert!(looks_like_aws_key(
        "AKIAIOSFODNN7EXAMPLE12345678901234567890"
    ));
    assert!(looks_like_aws_key(
        "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
    ));
}

#[test]
fn looks_like_aws_key_rejects_wrong_length() {
    assert!(!looks_like_aws_key("AKIAIOSFODNN7EXAMPLE"));
    assert!(!looks_like_aws_key("short"));
}

#[test]
fn looks_like_uuid_detects_uuid_format() {
    assert!(looks_like_uuid("550e8400-e29b-41d4-a716-446655440000"));
    assert!(looks_like_uuid("123e4567-e89b-12d3-a456-426614174000"));
}

#[test]
fn looks_like_uuid_rejects_non_uuid() {
    assert!(!looks_like_uuid("not-a-uuid"));
    assert!(!looks_like_uuid("550e8400-e29b-41d4-a716"));
}

#[test]
fn looks_like_uuid_rejects_wrong_segment_lengths() {
    assert!(!looks_like_uuid("550e84001-e29b-41d4-a71-446655440000"));
    assert!(!looks_like_uuid("550e8400-e29b-41d4-a716-44665544000g"));
}

#[test]
fn looks_like_secret_value_combines_all_checks() {
    assert!(looks_like_secret_value(
        "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.signature"
    ));
    assert!(looks_like_secret_value(
        "550e8400-e29b-41d4-a716-446655440000"
    ));
    assert!(looks_like_secret_value(
        "AKIAIOSFODNN7EXAMPLE12345678901234567890"
    ));
    assert!(looks_like_secret_value("AbCdEfGh123456789XyZ12345"));
}

#[test]
fn looks_like_secret_value_rejects_short_values() {
    assert!(!looks_like_secret_value("short"));
    assert!(!looks_like_secret_value("1234567890"));
}

#[test]
fn is_sensitive_env_key_case_insensitive() {
    assert!(is_sensitive_env_key("PASSWORD"));
    assert!(is_sensitive_env_key("password"));
    assert!(is_sensitive_env_key("PaSsWoRd"));
    assert!(is_sensitive_env_key("MY_API_KEY"));
    assert!(is_sensitive_env_key("my_api_key"));
}
