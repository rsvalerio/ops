//! `format_duration` formatting and SEC-15 input handling (NaN, negative,
//! infinite, enormous finite).

use super::*;

#[test]
fn zero_seconds() {
    assert_eq!(format_duration(0.0), "0.00s");
}

#[test]
fn sub_second() {
    assert_eq!(format_duration(0.74), "0.74s");
}

#[test]
fn whole_seconds() {
    assert_eq!(format_duration(5.37), "5.37s");
}

#[test]
fn just_under_a_minute() {
    assert_eq!(format_duration(59.99), "59.99s");
}

#[test]
fn exactly_sixty_seconds() {
    assert_eq!(format_duration(60.0), "1m0s");
}

#[test]
fn minutes_and_seconds() {
    assert_eq!(format_duration(134.0), "2m14s");
    assert_eq!(format_duration(278.04), "4m38s");
}

#[test]
fn exactly_one_hour() {
    assert_eq!(format_duration(3600.0), "1h0m0s");
}

#[test]
fn hours_minutes_seconds() {
    assert_eq!(format_duration(3723.0), "1h2m3s");
}

#[test]
fn large_duration() {
    assert_eq!(format_duration(7384.0), "2h3m4s");
}

#[test]
fn nan_input_renders_marker() {
    // SEC-15 / TASK-0358: NaN must not propagate through `as u64`.
    assert_eq!(format_duration(f64::NAN), "--");
}

#[test]
fn negative_input_renders_marker() {
    assert_eq!(format_duration(-1.0), "--");
    assert_eq!(format_duration(-3600.0), "--");
}

#[test]
fn infinity_renders_marker() {
    assert_eq!(format_duration(f64::INFINITY), "--");
    assert_eq!(format_duration(f64::NEG_INFINITY), "--");
}

#[test]
fn enormous_finite_input_does_not_panic() {
    let out = format_duration(1.0e30);
    assert!(out.ends_with('s'), "got: {out}");
    assert!(out.contains('h'), "got: {out}");
}

/// ERR-5 / TASK-0857: an enormous f64 (above the f64-representable u64
/// range) must clamp to the u64::MAX-derived hours/minutes/seconds form
/// without panicking under the new explicit clamp.
#[test]
fn enormous_finite_input_saturates_to_u64_max_form() {
    // f64 can't exactly represent u64::MAX, but `u64::MAX as f64` rounds
    // to 1.8446744073709552e19; passing that exact value must reach the
    // hours branch.
    let out = format_duration(u64::MAX as f64);
    assert!(out.contains('h'), "expected hours form, got: {out}");
}

/// ERR-5 / TASK-0857: an exact-fit large value (one second past the
/// 1-hour boundary, well within u64) round-trips through the same path.
#[test]
fn one_second_past_one_hour_is_one_hour() {
    assert_eq!(format_duration(3601.0), "1h0m1s");
}

/// ERR-5 / TASK-0857: an f64 between u64::MAX and infinity must still
/// clamp (no UB from out-of-range `as u64` cast).
#[test]
fn above_u64_max_finite_does_not_overflow() {
    let above = (u64::MAX as f64) * 2.0;
    assert!(above.is_finite(), "test premise");
    let out = format_duration(above);
    assert!(out.contains('h'), "expected hours form, got: {out}");
}
