//! Identifiers, all of them ULIDs.
//!
//! trdr needs a handful of identities that survive a backup, cross a socket, and
//! sit inside a file name or a Keychain account string without being escaped.
//! One representation answers all of it: a ULID, written as 26 characters of
//! Crockford base32.
//!
//! Why this and not a UUID or a path:
//!
//! - It sorts by creation time as plain text, which is what a manifest, a
//!   recovery point directory, and a log line all want.
//! - Its alphabet is `0-9` and `A-Z` minus `I`, `L`, `O`, `U`, so it never
//!   collides with the `:` that separates the fields of a Keychain account name
//!   (`docs/FOUNDATION_DESIGN.md` section 5.1), never needs quoting in a path,
//!   and cannot be misread between the digit and the letter.
//! - It is fixed length, so a parsed identifier is also a bounded one — nothing
//!   arriving over the socket can turn into an unbounded string.
//! - The first 48 bits are the creation time, so an id is self-describing
//!   without a lookup.
//!
//! Generating one needs a clock and a random source, which this crate must not
//! have. Values are built here from a [`Ulid`] the runtime supplies, and parsed
//! from text; that keeps `IdGenerator` a seam the runtime owns, as
//! `docs/FOUNDATION_DESIGN.md` section 13 requires.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

pub use ulid::Ulid;

/// The number of characters in the canonical text form of every id here.
pub const ID_LENGTH: usize = 26;

/// Text handed in was not a ULID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum IdParseError
{
    /// The text was not exactly [`ID_LENGTH`] characters long.
    #[error("expected {ID_LENGTH} characters, found {found}")]
    Length
    {
        /// How many characters arrived.
        found: usize
    },
    /// The text held a character outside the Crockford base32 alphabet.
    #[error("contains a character outside the ULID alphabet")]
    Alphabet,
    /// The text was 26 valid characters but named a number above 128 bits.
    ///
    /// The encoding has room for 130 bits, so `ZZZZ…` is spellable and is not a
    /// ULID. Accepting it would let two different strings mean one identifier.
    #[error("names a value larger than the 128 bits of a ULID")]
    Overflow
}

/// Declares one ULID-backed identifier type with its full text and serde
/// behaviour, so that no two ids in trdr can disagree about parsing.
macro_rules! ulid_id
{
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        ///
        /// Text form is 26 characters of Crockford base32. Parsing accepts
        /// either case and always renders back in upper case.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Ulid);

        impl $name
        {
            /// Wraps a ULID the runtime generated.
            pub const fn from_ulid(value: Ulid) -> Self
            {
                Self(value)
            }

            /// The ULID inside, for a caller that needs its timestamp.
            pub const fn into_ulid(self) -> Ulid
            {
                self.0
            }
        }

        impl FromStr for $name
        {
            type Err = IdParseError;

            fn from_str(text: &str) -> Result<Self, Self::Err>
            {
                parse_ulid(text).map(Self)
            }
        }

        impl fmt::Display for $name
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
            {
                let mut buffer = [0u8; ID_LENGTH];
                f.write_str(self.0.array_to_str(&mut buffer))
            }
        }

        /// Written out deliberately: the whole value is the identifier, and it
        /// carries nothing private, so `Debug` shows the same 26 characters
        /// `Display` does rather than the ULID's internal integer.
        impl fmt::Debug for $name
        {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
            {
                write!(f, concat!(stringify!($name), "({})"), self)
            }
        }

        impl Serialize for $name
        {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
            {
                let mut buffer = [0u8; ID_LENGTH];
                serializer.serialize_str(self.0.array_to_str(&mut buffer))
            }
        }

        impl<'de> Deserialize<'de> for $name
        {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
            {
                let text = String::deserialize(deserializer)?;
                text.parse().map_err(D::Error::custom)
            }
        }

        /// TypeScript sees a plain string — `str` rather than `String`, so
        /// the generated type is `string` and not a named alias. The validation
        /// is the Rust side's job and does not survive the crossing, which is
        /// why every id arriving from the WebView is parsed again here.
        impl specta::Type for $name
        {
            fn definition(types: &mut specta::Types) -> specta::datatype::DataType
            {
                <str as specta::Type>::definition(types)
            }
        }
    };
}

/// Parses the canonical text form, rejecting the two strings the underlying
/// codec would otherwise accept and silently change.
fn parse_ulid(text: &str) -> Result<Ulid, IdParseError>
{
    if text.len() != ID_LENGTH
    {
        return Err(IdParseError::Length {
            found: text.chars().count()
        });
    }

    let value = Ulid::from_string(text).map_err(|error| match error
    {
        ulid::DecodeError::InvalidLength => IdParseError::Length {
            found: text.chars().count()
        },
        ulid::DecodeError::InvalidChar => IdParseError::Alphabet
    })?;

    // The alphabet is checked by now, so a first character above `7` is a value
    // that does not fit. The codec keeps the low 128 bits of a 130-bit encoding
    // and would take `ZZZZ…` and print it back as `7ZZZ…`; something that does
    // not survive its own round trip is not an identifier.
    match text.as_bytes()[0]
    {
        b'0'..=b'7' => Ok(value),
        _ => Err(IdParseError::Overflow)
    }
}

ulid_id! {
    /// The portable logical identity of a workspace.
    ///
    /// It is written into `workspace.json` and travels with a backup, so two
    /// copies of one workspace on two Macs share it. The per-machine identity
    /// that Keychain items hang off is a different value and is not this one
    /// (`docs/FOUNDATION_DESIGN.md` section 5.1).
    WorkspaceId
}

ulid_id! {
    /// Correlates one request with its one response.
    ///
    /// Mandatory on every socket frame (section 9.2). A repeated id must get the
    /// same terminal result rather than a second execution, which is why it is a
    /// value the app can key on and not a free-form string.
    RequestId
}

ulid_id! {
    /// Ties an error to the chain of causes recorded in the local log.
    ///
    /// The chain itself stays in the log. Only this id crosses to a screen or a
    /// terminal, so no upstream body or credential can ride along with it.
    CauseChainId
}

ulid_id! {
    /// One pending human approval (section 9.3).
    ///
    /// The host mints it when it raises the native sheet, and the response names
    /// it. Nothing else can answer an approval.
    ApprovalRequestId
}

ulid_id! {
    /// A path the user picked, as the WebView is allowed to refer to it.
    ///
    /// Section 9.1 forbids the WebView from passing paths as strings. The native
    /// picker hands the host a real location and the host hands the WebView one
    /// of these; the mapping never leaves the host, so a screen cannot name a
    /// file the user did not choose.
    ScopedPathHandle
}

/// A user-facing identifier, such as the `low-vol-v1` in section 9.2's
/// `ui.open` example.
///
/// This is the name a person gave a strategy in their own file, so it is not a
/// ULID. It is still a value that arrives from outside and gets used to look
/// things up, so its shape is fixed here: 1 to 64 bytes, ASCII letters, digits,
/// `-`, `_`, and `.`, starting with a letter or a digit. That rules out path
/// separators, `..`, leading dashes, whitespace, and control characters.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(String);

/// The longest a [`ResourceId`] may be.
pub const RESOURCE_ID_MAX_BYTES: usize = 64;

/// Text handed in was not a usable resource identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ResourceIdError
{
    /// Empty, or longer than [`RESOURCE_ID_MAX_BYTES`].
    #[error("expected 1 to {RESOURCE_ID_MAX_BYTES} bytes, found {found}")]
    Length
    {
        /// How many bytes arrived.
        found: usize
    },
    /// Held a byte outside letters, digits, `-`, `_`, and `.`.
    #[error("contains a character that is not a letter, digit, '-', '_', or '.'")]
    Charset,
    /// Did not start with a letter or a digit.
    #[error("must start with a letter or a digit")]
    Start
}

impl ResourceId
{
    /// Checks the text and keeps it, or says which rule it broke.
    pub fn parse(text: impl Into<String>) -> Result<Self, ResourceIdError>
    {
        let text = text.into();

        if text.is_empty() || text.len() > RESOURCE_ID_MAX_BYTES
        {
            return Err(ResourceIdError::Length { found: text.len() });
        }

        if !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        {
            return Err(ResourceIdError::Charset);
        }

        match text.as_bytes()[0].is_ascii_alphanumeric()
        {
            true => Ok(Self(text)),
            false => Err(ResourceIdError::Start)
        }
    }

    /// The text, unchanged from what was accepted.
    pub fn as_str(&self) -> &str
    {
        &self.0
    }
}

impl FromStr for ResourceId
{
    type Err = ResourceIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err>
    {
        Self::parse(text)
    }
}

impl fmt::Display for ResourceId
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for ResourceId
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!(f, "ResourceId({})", self.0)
    }
}

impl Serialize for ResourceId
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error>
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ResourceId
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    {
        let text = String::deserialize(deserializer)?;
        Self::parse(text).map_err(D::Error::custom)
    }
}

impl specta::Type for ResourceId
{
    fn definition(types: &mut specta::Types) -> specta::datatype::DataType
    {
        <str as specta::Type>::definition(types)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    const SAMPLE: &str = "01KZNNR5X818P3J6ENYKSADP8W";

    #[test]
    fn canonical_text_round_trips_through_the_type()
    {
        let id: WorkspaceId = SAMPLE.parse().unwrap();
        assert_eq!(id.to_string(), SAMPLE);
        assert_eq!(SAMPLE.parse::<WorkspaceId>().unwrap(), id);
    }

    #[test]
    fn a_value_round_trips_through_text()
    {
        let id = WorkspaceId::from_ulid(Ulid(0x0192_3f5c_7d81_1234_5678_9abc_def0_1234));
        assert_eq!(id.to_string().parse::<WorkspaceId>().unwrap(), id);
    }

    #[test]
    fn round_trips_through_json()
    {
        let id: RequestId = SAMPLE.parse().unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{SAMPLE}\""));
        assert_eq!(serde_json::from_str::<RequestId>(&json).unwrap(), id);
    }

    #[test]
    fn lower_case_is_accepted_and_normalised_upward()
    {
        let lower: WorkspaceId = SAMPLE.to_lowercase().parse().unwrap();
        assert_eq!(lower.to_string(), SAMPLE);
        assert_eq!(lower, SAMPLE.parse::<WorkspaceId>().unwrap());
    }

    #[test]
    fn the_smallest_and_largest_ulids_survive_a_round_trip()
    {
        for text in ["00000000000000000000000000", "7ZZZZZZZZZZZZZZZZZZZZZZZZZ"]
        {
            assert_eq!(text.parse::<WorkspaceId>().unwrap().to_string(), text);
        }
    }

    #[test]
    fn a_value_the_codec_would_silently_truncate_is_refused()
    {
        // Both of these decode to a smaller number in the underlying codec and
        // would print back as something else.
        for text in ["ZZZZZZZZZZZZZZZZZZZZZZZZZZ", "8ZZZZZZZZZZZZZZZZZZZZZZZZZ"]
        {
            assert_eq!(text.parse::<WorkspaceId>(), Err(IdParseError::Overflow));
        }
    }

    #[test]
    fn the_wrong_length_is_refused()
    {
        assert_eq!(
            "".parse::<WorkspaceId>(),
            Err(IdParseError::Length { found: 0 })
        );
        assert_eq!(
            format!("{SAMPLE}0").parse::<WorkspaceId>(),
            Err(IdParseError::Length { found: 27 })
        );
    }

    #[test]
    fn the_ambiguous_letters_are_refused()
    {
        for letter in ['I', 'L', 'O', 'U', '-', '/']
        {
            let text = format!("{}{letter}", &SAMPLE[..ID_LENGTH - 1]);
            assert_eq!(text.parse::<WorkspaceId>(), Err(IdParseError::Alphabet));
        }
    }

    #[test]
    fn a_non_ulid_string_is_refused_by_serde_too()
    {
        assert!(serde_json::from_str::<WorkspaceId>("\"low-vol-v1\"").is_err());
        assert!(serde_json::from_str::<WorkspaceId>("1").is_err());
    }

    #[test]
    fn debug_prints_the_identifier_itself()
    {
        let id: ApprovalRequestId = SAMPLE.parse().unwrap();
        assert_eq!(format!("{id:?}"), format!("ApprovalRequestId({SAMPLE})"));
    }

    #[test]
    fn each_id_type_is_its_own_type()
    {
        // Not a runtime assertion so much as a compile-time one: a workspace id
        // and a request id are both ULIDs and must not be interchangeable.
        let workspace: WorkspaceId = SAMPLE.parse().unwrap();
        let request: RequestId = SAMPLE.parse().unwrap();
        assert_eq!(workspace.into_ulid(), request.into_ulid());
    }

    #[test]
    fn a_resource_id_round_trips()
    {
        let id = ResourceId::parse("low-vol-v1").unwrap();
        assert_eq!(id.as_str(), "low-vol-v1");
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"low-vol-v1\"");
        assert_eq!(serde_json::from_str::<ResourceId>(&json).unwrap(), id);
    }

    #[test]
    fn a_resource_id_refuses_what_would_reach_the_filesystem()
    {
        assert_eq!(
            ResourceId::parse("../etc/passwd"),
            Err(ResourceIdError::Charset)
        );
        assert_eq!(ResourceId::parse(".."), Err(ResourceIdError::Start));
        assert_eq!(ResourceId::parse("a/b"), Err(ResourceIdError::Charset));
        assert_eq!(ResourceId::parse("a\0b"), Err(ResourceIdError::Charset));
        assert_eq!(ResourceId::parse("a b"), Err(ResourceIdError::Charset));
        assert_eq!(ResourceId::parse("-lead"), Err(ResourceIdError::Start));
        assert_eq!(
            ResourceId::parse(""),
            Err(ResourceIdError::Length { found: 0 })
        );
        assert_eq!(
            ResourceId::parse("x".repeat(RESOURCE_ID_MAX_BYTES + 1)),
            Err(ResourceIdError::Length {
                found: RESOURCE_ID_MAX_BYTES + 1
            })
        );
    }
}
