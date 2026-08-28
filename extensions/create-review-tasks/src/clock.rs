//! UTC stamp for backlog task frontmatter.
//!
//! `created_date` needs only a UTC civil date plus minute resolution, but the
//! epoch → civil-date reduction that produces it is exactly the arithmetic
//! TIME-1 forbids hand-rolling, so it is delegated to `chrono`. That crate is
//! already compiled into the `ops` binary through `duckdb -> arrow ->
//! arrow-arith`, so depending on it directly costs no extra build time and no
//! new supply-chain surface.
//!
//! ERR-6: reading the clock is fallible and says so. When the host clock reads
//! before 1970-01-01 — a container started before NTP steps it, an RTC-less
//! board that boots at 0, a VM restored from a bad snapshot — [`UtcStamp::now`]
//! returns an error naming the clock instead of substituting the Unix epoch.
//! A silent substitution would date every task file, the
//! `review-request-<date>-<n>` title, and the sequence namespace those ids are
//! allocated in `1970-01-01`, unrecoverably and without any signal.

use anyhow::Context as _;

/// UTC calendar stamp at minute resolution, pre-formatted for frontmatter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UtcStamp {
    /// `YYYY-MM-DD`
    pub(crate) date: String,
    /// `HH:MM`
    pub(crate) minutes: String,
}

impl UtcStamp {
    /// Convert Unix seconds (UTC) into a [`UtcStamp`], or `None` when the
    /// instant lies outside the range `chrono` can represent (roughly year
    /// 262143 — unreachable for any wall-clock value, but the conversion
    /// refuses rather than clamping).
    pub(crate) fn from_unix_secs(secs: u64) -> Option<Self> {
        let at = chrono::DateTime::from_timestamp(i64::try_from(secs).ok()?, 0)?;
        Some(Self {
            date: at.format("%Y-%m-%d").to_string(),
            minutes: at.format("%H:%M").to_string(),
        })
    }

    /// Stamp for the current wall-clock time.
    ///
    /// # Errors
    ///
    /// The host clock reads before 1970-01-01, or so far after it that the
    /// instant has no calendar representation.
    pub(crate) fn now() -> anyhow::Result<Self> {
        Self::at(std::time::SystemTime::now())
    }

    /// [`UtcStamp::now`] against an explicit clock reading; the seam the
    /// pre-epoch test drives.
    ///
    /// # Errors
    ///
    /// As [`UtcStamp::now`].
    pub(crate) fn at(reading: std::time::SystemTime) -> anyhow::Result<Self> {
        let since_epoch = reading
            .duration_since(std::time::UNIX_EPOCH)
            .context("system clock reads before 1970-01-01; refusing to date review tasks")?;
        let secs = since_epoch.as_secs();
        Self::from_unix_secs(secs).with_context(|| {
            format!(
                "system clock reads {secs} seconds after 1970-01-01, which is not a \
                 representable date; refusing to date review tasks"
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The formatted stamp for `secs`, which every case below expects to be
    /// representable.
    fn stamp(secs: u64) -> UtcStamp {
        UtcStamp::from_unix_secs(secs).expect("timestamp must be representable")
    }

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
            let stamp = stamp(secs);
            assert_eq!(stamp.date, date, "date for epoch secs {secs}");
            assert_eq!(stamp.minutes, minutes, "minutes for epoch secs {secs}");
        }
    }

    /// Leap-year boundary: the day after 2024-02-29 must roll the month, not
    /// produce an impossible `2024-02-30`.
    #[test]
    fn day_after_leap_day_rolls_to_march() {
        assert_eq!(stamp(1_709_164_800 + 86_400).date, "2024-03-01");
    }

    /// Non-leap century boundary: 2100-02-28 → 2100-03-01 (2100 is divisible
    /// by 100 and not by 400, so it has no February 29).
    #[test]
    fn non_leap_century_skips_feb_29() {
        // 2100-02-28 00:00:00 UTC = 4_107_456_000 (verified against `date -u`).
        assert_eq!(stamp(4_107_456_000).date, "2100-02-28");
        assert_eq!(stamp(4_107_456_000 + 86_400).date, "2100-03-01");
    }

    /// Year boundary rollover (December 31 → January 1, year increments).
    #[test]
    fn year_rolls_at_december_midnight() {
        let stamp = stamp(1_767_225_540 + 60);
        assert_eq!(stamp.date, "2026-01-01");
        assert_eq!(stamp.minutes, "00:00");
    }

    /// An instant past the representable calendar range is refused, not
    /// clamped to some plausible-looking date.
    #[test]
    fn unrepresentable_timestamp_is_refused() {
        assert!(UtcStamp::from_unix_secs(u64::MAX).is_none());
    }

    /// ERR-6: a clock reading before the epoch is an error naming the clock,
    /// never a stamp dated 1970-01-01.
    #[test]
    fn pre_epoch_clock_is_an_error_naming_the_clock() {
        let reading = std::time::UNIX_EPOCH
            .checked_sub(std::time::Duration::from_secs(1))
            .expect("a pre-epoch SystemTime must be constructible");
        let err = UtcStamp::at(reading).expect_err("a pre-epoch clock must not produce a stamp");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("system clock reads before 1970-01-01"),
            "error must name the clock, got: {rendered}"
        );
    }

    /// The ordinary path still yields a stamp from the real clock.
    #[test]
    fn now_reads_the_host_clock() {
        let stamp = UtcStamp::now().expect("the host clock must be readable");
        assert_eq!(stamp.date.len(), 10, "date shape, got: {}", stamp.date);
        assert_eq!(
            stamp.minutes.len(),
            5,
            "minutes shape, got: {}",
            stamp.minutes
        );
    }
}
