//! The one way trdr writes an instant down.
//!
//! Section 7.2 of `docs/FOUNDATION_DESIGN.md` requires a canonical form for
//! every datetime that goes into a hash, and section 8.1 shows it: RFC 3339 in
//! UTC, `2026-08-10T10:00:00Z`. Two spellings of one instant would give two
//! hashes for one backtest, so this type accepts exactly one spelling per
//! instant and refuses local offsets, lower-case `t`, and a missing zone.
//!
//! What this is not: a clock, a calendar, or arithmetic. It holds text that has
//! been checked, and reading the current time stays a runtime seam. The meaning
//! of the four timestamp fields in section 7.1 — observed, available, effective,
//! recorded — belongs to the domain track that introduces them.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

/// An instant in UTC, in RFC 3339 form.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(String);

/// Text was not a canonical UTC timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TimestampError
{
    /// Not shaped like `YYYY-MM-DDTHH:MM:SSZ`, with optional fractional seconds.
    #[error("expected the form YYYY-MM-DDTHH:MM:SS[.fff]Z")]
    Shape,
    /// Shaped right, but naming a date or time that does not exist.
    #[error("names a date or time that does not exist")]
    OutOfRange,
    /// Carried a zone other than `Z`.
    ///
    /// A local offset is a second spelling of an instant that already has one.
    #[error("must be written in UTC, ending in 'Z'")]
    NotUtc
}

impl Timestamp
{
    /// Checks the text and keeps it, or says which rule it broke.
    pub fn parse(text: impl Into<String>) -> Result<Self, TimestampError>
    {
        let text = text.into();
        check(&text)?;
        Ok(Self(text))
    }

    /// The text, unchanged from what was accepted.
    pub fn as_str(&self) -> &str
    {
        &self.0
    }
}

/// Runs every rule the form has to satisfy.
fn check(text: &str) -> Result<(), TimestampError>
{
    let body = text.strip_suffix('Z').ok_or(TimestampError::NotUtc)?;
    let (date_time, fraction) = match body.split_once('.')
    {
        Some((head, tail)) => (head, Some(tail)),
        None => (body, None)
    };

    if let Some(fraction) = fraction
    {
        let digits = fraction.len();

        if !(1..=9).contains(&digits) || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(TimestampError::Shape);
        }
    }

    let (date, time) = date_time.split_once('T').ok_or(TimestampError::Shape)?;
    check_date(date)?;
    check_time(time)
}

/// Checks `YYYY-MM-DD`, including how long the month actually is.
fn check_date(date: &str) -> Result<(), TimestampError>
{
    let parts: Vec<&str> = date.split('-').collect();

    if parts.len() != 3 || [4, 2, 2] != [parts[0].len(), parts[1].len(), parts[2].len()]
    {
        return Err(TimestampError::Shape);
    }

    let year = number(parts[0])?;
    let month = number(parts[1])?;
    let day = number(parts[2])?;

    match (1..=12).contains(&month) && (1..=days_in_month(year, month)).contains(&day)
    {
        true => Ok(()),
        false => Err(TimestampError::OutOfRange)
    }
}

/// Checks `HH:MM:SS`, allowing second 60 for a leap second.
fn check_time(time: &str) -> Result<(), TimestampError>
{
    let parts: Vec<&str> = time.split(':').collect();

    if parts.len() != 3 || parts.iter().any(|part| part.len() != 2)
    {
        return Err(TimestampError::Shape);
    }

    let hour = number(parts[0])?;
    let minute = number(parts[1])?;
    let second = number(parts[2])?;

    match hour <= 23 && minute <= 59 && second <= 60
    {
        true => Ok(()),
        false => Err(TimestampError::OutOfRange)
    }
}

/// Reads a run of ASCII digits, refusing anything else a number parser would
/// otherwise take, such as `+1` or a Unicode digit.
fn number(text: &str) -> Result<u32, TimestampError>
{
    match text.bytes().all(|b| b.is_ascii_digit())
    {
        true => text.parse().map_err(|_| TimestampError::Shape),
        false => Err(TimestampError::Shape)
    }
}

/// How many days that month has, in that year.
fn days_in_month(year: u32, month: u32) -> u32
{
    match month
    {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) =>
        {
            29
        }
        2 => 28,
        _ => 0
    }
}

impl FromStr for Timestamp
{
    type Err = TimestampError;

    fn from_str(text: &str) -> Result<Self, Self::Err>
    {
        Self::parse(text)
    }
}

impl fmt::Display for Timestamp
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for Timestamp
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!(f, "Timestamp({})", self.0)
    }
}

impl Serialize for Timestamp
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Timestamp
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    {
        let text = String::deserialize(deserializer)?;
        Self::parse(text).map_err(D::Error::custom)
    }
}

impl specta::Type for Timestamp
{
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType
    {
        <String as specta::Type>::definition(types)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn the_documented_form_round_trips()
    {
        let text = "2026-08-10T10:00:00Z";
        let stamp = Timestamp::parse(text).unwrap();
        assert_eq!(stamp.as_str(), text);
        assert_eq!(
            serde_json::to_string(&stamp).unwrap(),
            format!("\"{text}\"")
        );
        assert_eq!(
            serde_json::from_str::<Timestamp>(&format!("\"{text}\"")).unwrap(),
            stamp
        );
    }

    #[test]
    fn fractional_seconds_are_allowed()
    {
        assert!(Timestamp::parse("2026-08-10T10:00:00.123Z").is_ok());
        assert!(Timestamp::parse("2026-08-10T10:00:00.123456789Z").is_ok());
        assert_eq!(
            Timestamp::parse("2026-08-10T10:00:00.Z"),
            Err(TimestampError::Shape)
        );
    }

    #[test]
    fn a_second_spelling_of_one_instant_is_refused()
    {
        assert_eq!(
            Timestamp::parse("2026-08-08T00:00:00+09:00"),
            Err(TimestampError::NotUtc)
        );
        assert_eq!(
            Timestamp::parse("2026-08-10t10:00:00Z"),
            Err(TimestampError::Shape)
        );
        assert_eq!(
            Timestamp::parse("2026-08-10 10:00:00Z"),
            Err(TimestampError::Shape)
        );
        assert_eq!(
            Timestamp::parse("2026-08-10T10:00:00"),
            Err(TimestampError::NotUtc)
        );
    }

    #[test]
    fn a_date_that_does_not_exist_is_refused()
    {
        assert_eq!(
            Timestamp::parse("2026-02-29T00:00:00Z"),
            Err(TimestampError::OutOfRange)
        );
        assert!(Timestamp::parse("2024-02-29T00:00:00Z").is_ok());
        assert!(Timestamp::parse("2000-02-29T00:00:00Z").is_ok());
        assert_eq!(
            Timestamp::parse("1900-02-29T00:00:00Z"),
            Err(TimestampError::OutOfRange)
        );
        assert_eq!(
            Timestamp::parse("2026-13-01T00:00:00Z"),
            Err(TimestampError::OutOfRange)
        );
        assert_eq!(
            Timestamp::parse("2026-00-01T00:00:00Z"),
            Err(TimestampError::OutOfRange)
        );
        assert_eq!(
            Timestamp::parse("2026-04-31T00:00:00Z"),
            Err(TimestampError::OutOfRange)
        );
    }

    #[test]
    fn a_time_that_does_not_exist_is_refused()
    {
        assert_eq!(
            Timestamp::parse("2026-08-10T24:00:00Z"),
            Err(TimestampError::OutOfRange)
        );
        assert_eq!(
            Timestamp::parse("2026-08-10T10:60:00Z"),
            Err(TimestampError::OutOfRange)
        );
        assert!(Timestamp::parse("2026-06-30T23:59:60Z").is_ok());
        assert_eq!(
            Timestamp::parse("2026-08-10T10:00:61Z"),
            Err(TimestampError::OutOfRange)
        );
    }

    #[test]
    fn a_shape_a_lenient_parser_would_take_is_refused()
    {
        for text in [
            "2026-8-10T10:00:00Z",
            "26-08-10T10:00:00Z",
            "2026-08-10T10:00Z",
            "Z"
        ]
        {
            assert_eq!(Timestamp::parse(text), Err(TimestampError::Shape), "{text}");
        }
        assert_eq!(
            Timestamp::parse("2026-08-+1T10:00:00Z"),
            Err(TimestampError::Shape)
        );
        assert_eq!(Timestamp::parse(""), Err(TimestampError::NotUtc));
    }
}
