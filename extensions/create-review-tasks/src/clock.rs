//! UTC stamp for backlog task frontmatter, with no time crate dependency.
//!
//! The workspace deliberately avoids `chrono` / `time`; `created_date` needs
//! only a UTC civil date plus minute resolution, so this module implements
//! the standard days-since-epoch → (year, month, day) reduction (Howard
//! Hinnant's `civil_from_days`) in plain `u64` arithmetic.

/// UTC calendar stamp at minute resolution, pre-formatted for frontmatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtcStamp {
    /// `YYYY-MM-DD`
    pub(crate) date: String,
    /// `HH:MM`
    pub(crate) minutes: String,
}

impl UtcStamp {
    /// Convert Unix seconds (UTC) into a [`UtcStamp`].
    ///
    /// The input contract is `SystemTime::now()`-shaped: non-negative and
    /// far below the `u64` overflow horizon (max representable days ≈ 5.8e11
    /// years), so the arithmetic below cannot overflow for any wall-clock
    /// value this CLI can observe.
    pub(crate) fn from_unix_secs(secs: u64) -> Self {
        let days = secs / 86_400;
        let day_secs = secs % 86_400;
        let (year, month, day) = civil_from_days(days);
        let hour = day_secs / 3_600;
        let minute = (day_secs % 3_600) / 60;

        // PERF-13: build both fields with `write!` into the destination
        // String instead of `format!` intermediates.
        use std::fmt::Write as _;
        let mut date = String::new();
        // `fmt::Write for String` never returns `Err`, so there is nothing to
        // report and nothing to panic on.
        let _ = write!(date, "{year:04}-{month:02}-{day:02}");
        let mut minutes = String::new();
        let _ = write!(minutes, "{hour:02}:{minute:02}");
        Self { date, minutes }
    }

    /// Stamp for the current wall-clock time.
    #[must_use = "reading the clock and discarding the stamp is always a bug"]
    pub(crate) fn now() -> Self {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        Self::from_unix_secs(secs)
    }
}

/// Days since 1970-01-01 → `(year, month, day)` in the proleptic Gregorian
/// calendar (Howard Hinnant, "chrono-Compatible Dates"). `u64` arithmetic is
/// sufficient because the input is non-negative by construction.
///
/// Every operation below is spelled `saturating_*` so the reduction is total,
/// and every one of them is *exactly* the checked result for the inputs this
/// module can produce. `days` comes from `secs / 86_400`, so it is at most
/// `u64::MAX / 86_400` (~2.1e14) and the epoch shift cannot overflow; each
/// product stays far below `u64::MAX` because `era`, `yoe`, `doy` and `mp` are
/// bounded by the ranges annotated on their lines; and each subtraction's right
/// operand is provably no larger than its left one — the invariants Hinnant's
/// derivation establishes (`doe <= 146_096` makes `doe / 146_096 <= 1` while
/// `doe - doe / 1_460 + doe / 36_524 >= 146_000` there, `yoe <= 399` makes the
/// day-count of whole years before `yoe` at most `doe`, and `mp <= 11` makes
/// `(153 * mp + 2) / 5 <= 337 <= doy` whenever `mp` was derived from `doy`).
const fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days.saturating_add(719_468);
    let era = z / 146_097;
    let doe = z % 146_097; // [0, 146096]
    let yoe_numerator = doe
        .saturating_sub(doe / 1_460)
        .saturating_add(doe / 36_524)
        .saturating_sub(doe / 146_096);
    let yoe = yoe_numerator / 365; // [0, 399]
    let y = yoe.saturating_add(era.saturating_mul(400));
    // Days occupied by the whole years preceding `yoe` within this era.
    let days_before_yoe = yoe
        .saturating_mul(365)
        .saturating_add(yoe / 4)
        .saturating_sub(yoe / 100);
    let doy = doe.saturating_sub(days_before_yoe); // [0, 365]
    let mp = doy.saturating_mul(5).saturating_add(2) / 153; // [0, 11]
    let d = doy
        .saturating_sub(mp.saturating_mul(153).saturating_add(2) / 5)
        .saturating_add(1); // [1, 31]
    let m = if mp < 10 {
        mp.saturating_add(3)
    } else {
        mp.saturating_sub(9)
    }; // [1, 12]
    let y = if m <= 2 { y.saturating_add(1) } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TEST-11: pin the full formatted stamp, not just `is_some`-style
    /// structural checks. Expected values verified against `date -u -d @…`.
    #[test]
    fn from_unix_secs_formats_known_timestamps() {
        let cases: &[(u64, &str, &str)] = &[
            (0, "1970-01-01", "00:00"),
            (3_599, "1970-01-01", "00:59"),
            (3_600, "1970-01-01", "01:00"),
            (1_000_000_000, "2001-09-09", "01:46"),
            (1_709_164_800, "2024-02-29", "00:00"),
            (1_767_225_540, "2025-12-31", "23:59"),
        ];
        for &(secs, date, minutes) in cases {
            let stamp = UtcStamp::from_unix_secs(secs);
            assert_eq!(stamp.date, date, "date for epoch secs {secs}");
            assert_eq!(stamp.minutes, minutes, "minutes for epoch secs {secs}");
        }
    }

    /// Leap-year boundary: the day after 2024-02-29 must roll the month, not
    /// produce an impossible `2024-02-30`.
    #[test]
    fn day_after_leap_day_rolls_to_march() {
        let stamp = UtcStamp::from_unix_secs(1_709_164_800 + 86_400);
        assert_eq!(stamp.date, "2024-03-01");
    }

    /// Non-leap century boundary: 2100-02-28 → 2100-03-01 (2100 is divisible
    /// by 100 and not by 400, so it has no February 29).
    #[test]
    fn non_leap_century_skips_feb_29() {
        // 2100-02-28 00:00:00 UTC = 4_107_456_000 (verified against `date -u`).
        let feb_28 = UtcStamp::from_unix_secs(4_107_456_000);
        assert_eq!(feb_28.date, "2100-02-28");
        let next = UtcStamp::from_unix_secs(4_107_456_000 + 86_400);
        assert_eq!(next.date, "2100-03-01");
    }

    /// Year boundary rollover (December 31 → January 1, year increments).
    #[test]
    fn year_rolls_at_december_midnight() {
        let stamp = UtcStamp::from_unix_secs(1_767_225_540 + 60);
        assert_eq!(stamp.date, "2026-01-01");
        assert_eq!(stamp.minutes, "00:00");
    }
}
