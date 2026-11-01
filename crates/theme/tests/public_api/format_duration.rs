//! `format_duration` formatting and input handling (NaN, negative,
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
    // NaN must not propagate through `as u64`.
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

/// An enormous f64 (above the f64-representable u64
/// range) must clamp to the `u64::MAX-derived` hours/minutes/seconds form
/// without panicking under the new explicit clamp.
#[test]
fn enormous_finite_input_saturates_to_u64_max_form() {
    // f64 can't exactly represent u64::MAX, but `u64::MAX as f64` rounds
    // to 1.8446744073709552e19; passing that exact value must reach the
    // hours branch.
    // The `as` is the assertion: this test pins the exact clamp bound
    // `format_duration` computes with `u64::MAX as f64`, so it must spell the
    // conversion the same rounding-up way. `f64::from`/`try_from` do not exist
    // for u64, and hard-coding 1.8446744073709552e19 would silently drift from
    // the production bound.
    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
    let out = format_duration(u64::MAX as f64);
    assert!(out.contains('h'), "expected hours form, got: {out}");
}

/// An exact-fit large value (one second past the
/// 1-hour boundary, well within u64) round-trips through the same path.
#[test]
fn one_second_past_one_hour_is_one_hour() {
    assert_eq!(format_duration(3601.0), "1h0m1s");
}

/// An f64 between `u64::MAX` and infinity must still
/// clamp (no UB from out-of-range `as u64` cast).
#[test]
fn above_u64_max_finite_does_not_overflow() {
    // Same reason as above: the test's premise is "twice the exact f64 clamp
    // bound `format_duration` uses", which is only expressible as
    // `u64::MAX as f64` — there is no `From`/`TryFrom` impl for u64 -> f64.
    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
    let above = (u64::MAX as f64) * 2.0;
    assert!(above.is_finite(), "test premise");
    let out = format_duration(above);
    assert!(out.contains('h'), "expected hours form, got: {out}");
}

/// `format_running_elapsed` is `format_duration` under an hour.
#[test]
fn running_elapsed_matches_format_duration_under_an_hour() {
    for secs in [0.0, 0.74, 59.99, 60.0, 134.0, 3599.0] {
        assert_eq!(format_running_elapsed(secs), format_duration(secs));
    }
}

/// Past an hour a running time drops its seconds instead of growing past
/// the six-column `{elapsed:>6}` field: `format_duration` prints `1h0m10s`
/// (7 columns) and `1h10m10s` (8), which wrapped hour-long running rows.
#[test]
fn running_elapsed_past_an_hour_is_hours_then_days() {
    assert_eq!(format_running_elapsed(3600.0), "1h00m");
    assert_eq!(format_running_elapsed(3610.0), "1h00m");
    assert_eq!(format_running_elapsed(4210.0), "1h10m");
    assert_eq!(format_running_elapsed(35_999.0), "9h59m");
    assert_eq!(format_running_elapsed(359_999.0), "99h59m");
    assert_eq!(format_running_elapsed(360_000.0), "4d04h");
    assert_eq!(format_running_elapsed(8_639_999.0), "99d23h");
    assert_eq!(format_running_elapsed(8_640_000.0), "100d");
}

#[test]
fn running_elapsed_rejects_non_finite_and_negative_input() {
    assert_eq!(format_running_elapsed(f64::NAN), "--");
    assert_eq!(format_running_elapsed(f64::INFINITY), "--");
    assert_eq!(format_running_elapsed(-1.0), "--");
}

proptest::proptest! {
    /// Every running time up to 99999 days fits the six-column field.
    #[test]
    fn running_elapsed_fits_the_running_field(secs in 0.0f64..8_639_913_600.0) {
        let formatted = format_running_elapsed(secs);
        proptest::prop_assert!(
            formatted.chars().count() <= RUNNING_ELAPSED_WIDTH,
            "{secs}s formatted as {formatted:?}"
        );
    }
}
