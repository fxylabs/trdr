//! The version field every trdr message carries.
//!
//! `docs/FOUNDATION_DESIGN.md` section 9.2 makes the protocol version and the
//! request id mandatory on every frame, and requires an unknown version to be
//! refused rather than guessed at. That refusal has to be a value a caller can
//! branch on, so the check lives here and produces [`UnsupportedVersion`], not a
//! free-text parse failure.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// The only wire version this binary speaks.
pub const PROTOCOL_VERSION: u16 = 1;

/// The `v` field of a versioned message.
///
/// A value of this type can only exist for a version this binary supports, so
/// code downstream of parsing never has to re-check it.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, specta::Type)]
pub struct EnvelopeVersion(u16);

impl EnvelopeVersion
{
    /// The version this binary writes.
    pub const CURRENT: Self = Self(PROTOCOL_VERSION);

    /// Accepts a raw version number, or reports the one it saw.
    pub fn supported(raw: u16) -> Result<Self, UnsupportedVersion>
    {
        if raw == PROTOCOL_VERSION
        {
            Ok(Self(raw))
        }
        else
        {
            Err(UnsupportedVersion {
                found: raw,
                supported: PROTOCOL_VERSION
            })
        }
    }

    /// The raw number, for putting back on the wire.
    pub const fn get(self) -> u16
    {
        self.0
    }
}

impl Default for EnvelopeVersion
{
    fn default() -> Self
    {
        Self::CURRENT
    }
}

impl fmt::Debug for EnvelopeVersion
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!(f, "EnvelopeVersion({})", self.0)
    }
}

impl fmt::Display for EnvelopeVersion
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!(f, "{}", self.0)
    }
}

impl Serialize for EnvelopeVersion
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        self.0.serialize(serializer)
    }
}

/// Deserialising is the second gate on the version.
///
/// The frame parsers in [`crate::socket`] check the version first so that the
/// caller gets [`UnsupportedVersion`] with the number in it. This impl exists so
/// that a message deserialised by any other route — a Tauri command argument,
/// for instance — still cannot carry a version this binary does not speak.
impl<'de> Deserialize<'de> for EnvelopeVersion
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    {
        let raw = u16::deserialize(deserializer)?;
        Self::supported(raw).map_err(D::Error::custom)
    }
}

/// A message arrived with a protocol version this binary does not speak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unsupported protocol version {found}, this build speaks {supported}")]
pub struct UnsupportedVersion
{
    /// The version the message claimed.
    pub found: u16,
    /// The version this binary speaks.
    pub supported: u16
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn current_version_round_trips_as_a_bare_number()
    {
        let json = serde_json::to_string(&EnvelopeVersion::CURRENT).unwrap();
        assert_eq!(json, "1");
        assert_eq!(
            serde_json::from_str::<EnvelopeVersion>(&json).unwrap(),
            EnvelopeVersion::CURRENT
        );
    }

    #[test]
    fn an_unsupported_version_is_a_typed_error()
    {
        let refused = EnvelopeVersion::supported(2).unwrap_err();
        assert_eq!(
            refused,
            UnsupportedVersion {
                found: 2,
                supported: 1
            }
        );
    }

    #[test]
    fn deserialising_refuses_an_unsupported_version()
    {
        assert!(serde_json::from_str::<EnvelopeVersion>("2").is_err());
        assert!(serde_json::from_str::<EnvelopeVersion>("0").is_err());
    }

    #[test]
    fn debug_shows_the_version_and_nothing_else()
    {
        assert_eq!(
            format!("{:?}", EnvelopeVersion::CURRENT),
            "EnvelopeVersion(1)"
        );
        assert_eq!(EnvelopeVersion::CURRENT.to_string(), "1");
    }
}
