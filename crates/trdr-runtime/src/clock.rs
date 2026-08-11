//! Reading the current instant, behind the seam section 13 calls `Clock`.
//!
//! `trdr-core` can hold a [`Timestamp`] and check that it is written the one way
//! section 7.2 allows, but it cannot produce one: reading a clock is an effect,
//! and the domain crate has no way to reach the outside world. So the clock is a
//! trait here, the operating system's clock is one implementation of it, and a
//! test substitutes its own (see [`crate::test_support::FixedClock`]).
//!
//! # Why the calendar arithmetic is written out
//!
//! Turning a count of seconds into `2026-08-11T07:14:03Z` needs a proleptic
//! Gregorian calendar, and the obvious way to get one is another dependency. The
//! conversion is about twenty lines of well-known integer arithmetic, it has no
//! configuration, no locale, and no time zone database behind it, and every case
//! it can produce is covered below. A dependency here would be larger than the
//! thing it replaced.

use std::time::{SystemTime, UNIX_EPOCH};
use trdr_core::time::Timestamp;

/// How many seconds a day has. No leap seconds: `SystemTime` does not have them
/// either, so introducing one here would invent a difference rather than record
/// it.
const SECONDS_PER_DAY: i64 = 86_400;

/// Days from the Unix epoch back to `0000-01-01`, the earliest date a
/// [`Timestamp`] can spell with four digits of year.
const FIRST_DAY: i64 = -719_528;

/// Days from the Unix epoch forward to `9999-12-31`, the latest one.
const LAST_DAY: i64 = 2_932_896;

/// Where the current instant comes from.
///
/// `Send + Sync` because the app hands one to start-up and start-up is not the
/// only thread that will eventually want the time.
pub trait Clock: Send + Sync
{
    /// Now, in the one form section 7.2 fixes.
    fn now(&self) -> Timestamp;
}

/// The operating system's clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock
{
    fn now(&self) -> Timestamp
    {
        timestamp_at(SystemTime::now())
    }
}

/// The instant a [`SystemTime`] names, written the way section 7.2 fixes.
///
/// A machine whose clock is set outside the years 0 to 9999 gets the nearest
/// instant inside them rather than a failure. That is not a guess about what the
/// user meant: it is the only value of this type that exists, and refusing to
/// start over a clock nobody can read is worse than recording a bound.
pub fn timestamp_at(instant: SystemTime) -> Timestamp
{
    let seconds = match instant.duration_since(UNIX_EPOCH)
    {
        Ok(since) => since.as_secs() as i64,
        Err(before) => -(before.duration().as_secs() as i64)
    };

    let days = seconds
        .div_euclid(SECONDS_PER_DAY)
        .clamp(FIRST_DAY, LAST_DAY);
    let within_day = seconds.rem_euclid(SECONDS_PER_DAY);
    let (year, month, day) = civil_from_days(days);

    let text = format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        within_day / 3600,
        (within_day / 60) % 60,
        within_day % 60
    );

    // Unreachable by construction: the day is clamped into a range whose ends
    // are asserted below, and the time of day is a remainder of 86 400. The
    // parse is still here rather than a raw constructor because `Timestamp` has
    // no other way in, which is the property that keeps section 7.2 true.
    Timestamp::parse(text).expect("the clock renders a canonical timestamp")
}

/// The civil date a day number names, counting from the Unix epoch.
///
/// Howard Hinnant's `civil_from_days`, which is exact for the whole range of
/// `i64` days and needs no table.
fn civil_from_days(days: i64) -> (i64, i64, i64)
{
    // Shift the epoch to 0000-03-01, which puts the leap day at the end of the
    // year and makes the month arithmetic a single division.
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;

    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = match shifted_month < 10
    {
        true => shifted_month + 3,
        false => shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);

    (year, month, day)
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::time::Duration;

    fn at(unix_seconds: i64) -> Timestamp
    {
        match unix_seconds >= 0
        {
            true => timestamp_at(UNIX_EPOCH + Duration::from_secs(unix_seconds as u64)),
            false => timestamp_at(UNIX_EPOCH - Duration::from_secs(-unix_seconds as u64))
        }
    }

    #[test]
    fn the_epoch_is_written_the_way_section_seven_fixes()
    {
        assert_eq!(at(0).as_str(), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_known_instant_matches_the_one_the_design_uses_as_its_example()
    {
        // `date -u -j -f '%Y-%m-%dT%H:%M:%SZ' 2026-08-10T10:00:00Z +%s`
        assert_eq!(at(1_786_356_000).as_str(), "2026-08-10T10:00:00Z");
    }

    #[test]
    fn a_leap_day_is_a_day()
    {
        // 2024-02-29T12:34:56Z.
        assert_eq!(at(1_709_210_096).as_str(), "2024-02-29T12:34:56Z");
    }

    #[test]
    fn an_instant_before_the_epoch_counts_backwards()
    {
        assert_eq!(at(-1).as_str(), "1969-12-31T23:59:59Z");
        assert_eq!(at(-SECONDS_PER_DAY).as_str(), "1969-12-31T00:00:00Z");
    }

    /// The two constants the clamp rests on, checked rather than trusted. If
    /// either is wrong the clamp lets through a year `Timestamp` cannot spell,
    /// and [`timestamp_at`] panics on a machine nobody was looking at.
    #[test]
    fn the_clamp_ends_are_the_dates_they_claim_to_be()
    {
        assert_eq!(civil_from_days(FIRST_DAY), (0, 1, 1));
        assert_eq!(civil_from_days(LAST_DAY), (9999, 12, 31));
    }

    #[test]
    fn a_clock_set_outside_the_years_a_timestamp_can_spell_is_bounded()
    {
        assert_eq!(
            at(FIRST_DAY * SECONDS_PER_DAY - 1).as_str(),
            "0000-01-01T23:59:59Z"
        );
        assert_eq!(
            at(LAST_DAY * SECONDS_PER_DAY + SECONDS_PER_DAY * 400).as_str(),
            "9999-12-31T00:00:00Z"
        );
    }

    /// Every day from 1969 to 2100 rendered and parsed back, so the arithmetic
    /// is covered across century and leap boundaries rather than at four points.
    #[test]
    fn every_day_across_a_century_and_a_half_renders_something_canonical()
    {
        let mut previous = String::new();

        for day in -366..47_500
        {
            let rendered = at(day * SECONDS_PER_DAY + 43_200).as_str().to_owned();

            assert!(
                rendered > previous,
                "{rendered} did not come after {previous}"
            );
            assert!(rendered.ends_with("T12:00:00Z"), "{rendered}");
            previous = rendered;
        }
    }

    #[test]
    fn the_system_clock_answers_with_a_time_in_this_century()
    {
        let now = SystemClock.now();

        assert!(now.as_str() > "2020-01-01T00:00:00Z", "{now:?}");
        assert!(now.as_str() < "2100-01-01T00:00:00Z", "{now:?}");
    }
}
