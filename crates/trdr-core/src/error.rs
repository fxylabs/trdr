//! The one shape every trdr failure takes.
//!
//! `docs/FOUNDATION_DESIGN.md` section 12 fixes the families of error code and
//! says an error exposes a code, safe message parameters, whether retrying is
//! worthwhile, and an id pointing at the cause chain in the local log — and
//! nothing else. There is deliberately no free-text message field on
//! [`ErrorEnvelope`]: the screen and the CLI each render their own words from
//! the code, so an error can never carry a sentence that leaked an HTTP body, a
//! credential, an account number, or terminal bytes.
//!
//! The other rule this file exists to enforce is that one failure does not stand
//! in for another. A client that folds every non-success upstream reply into a
//! single "request failed" makes "your key is wrong" and "the broker is down"
//! indistinguishable, and the person reading the screen cannot tell whether to
//! fix something or wait. So [`ErrorCode`] keeps those apart, and
//! [`UpstreamStatus`] keeps what the upstream actually said next to it.

use crate::envelope::EnvelopeVersion;
use crate::id::{CauseChainId, RequestId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The longest text a single error parameter may carry.
///
/// Parameters name things — a collector, a file, a count. Anything longer than
/// this is a payload, and payloads do not belong in an error.
pub const MAX_PARAM_TEXT_BYTES: usize = 256;

/// The longest an upstream's own status string may be.
pub const MAX_UPSTREAM_CODE_BYTES: usize = 64;

/// A failure, in the form every surface receives it.
///
/// Built through [`ErrorEnvelope::new`] and the `with_*` methods so that the
/// version is always set and the parameter rules always run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ErrorEnvelope
{
    /// Protocol version, the same field the IPC frames carry.
    pub v: EnvelopeVersion,
    /// The request this failure answers, when it answers one.
    ///
    /// Absent for a failure that happened outside a request, such as one during
    /// start-up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
    /// What went wrong, as a value a caller can branch on.
    pub code: ErrorCode,
    /// Named values the surface may put into its own sentence.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, ErrorParam>,
    /// Whether trying again could work.
    pub retryability: Retryability,
    /// Points at the full cause chain in the local log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cause_chain_id: Option<CauseChainId>,
    /// What the upstream said, when an upstream said anything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<UpstreamStatus>
}

impl ErrorEnvelope
{
    /// A failure with the given code, not retryable, carrying nothing else.
    pub fn new(code: ErrorCode) -> Self
    {
        Self {
            v: EnvelopeVersion::CURRENT,
            id: None,
            code,
            params: BTreeMap::new(),
            retryability: Retryability::No,
            cause_chain_id: None,
            upstream: None
        }
    }

    /// Ties this failure to the request that produced it.
    pub fn with_request(mut self, id: RequestId) -> Self
    {
        self.id = Some(id);
        self
    }

    /// Adds one named value for the surface to render.
    pub fn with_param(mut self, name: impl Into<String>, value: ErrorParam) -> Self
    {
        self.params.insert(name.into(), value);
        self
    }

    /// Says whether and when retrying is worth it.
    pub fn with_retryability(mut self, retryability: Retryability) -> Self
    {
        self.retryability = retryability;
        self
    }

    /// Points at the cause chain kept in the local log.
    pub fn with_cause_chain(mut self, id: CauseChainId) -> Self
    {
        self.cause_chain_id = Some(id);
        self
    }

    /// Keeps what the upstream reported alongside trdr's own code.
    pub fn with_upstream(mut self, upstream: UpstreamStatus) -> Self
    {
        self.upstream = Some(upstream);
        self
    }
}

/// A value an error carries for a surface to put into its own sentence.
///
/// The variants are small on purpose. There is no "any JSON" variant, because
/// that is how an upstream response body ends up on a screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ErrorParam
{
    /// A short piece of text, at most [`MAX_PARAM_TEXT_BYTES`] bytes.
    Text(String),
    /// A whole number, such as a count or a limit.
    Integer(i64),
    /// A flag.
    Boolean(bool)
}

impl ErrorParam
{
    /// Accepts short text, or refuses it for being payload-sized.
    pub fn text(value: impl Into<String>) -> Result<Self, ParamTooLong>
    {
        let value = value.into();

        match value.len() <= MAX_PARAM_TEXT_BYTES
        {
            true => Ok(Self::Text(value)),
            false => Err(ParamTooLong {
                found: value.len(),
                limit: MAX_PARAM_TEXT_BYTES
            })
        }
    }

    /// Text from a string the program itself wrote.
    ///
    /// For the fixed words the code supplies, where the length is not in
    /// question and a `Result` would only be noise at the call site.
    pub fn literal(value: &'static str) -> Self
    {
        debug_assert!(
            value.len() <= MAX_PARAM_TEXT_BYTES,
            "a literal parameter is over the limit"
        );
        Self::Text(value.to_owned())
    }
}

/// A parameter was longer than an error parameter is allowed to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("parameter is {found} bytes, the limit is {limit}")]
pub struct ParamTooLong
{
    /// How long the text was.
    pub found: usize,
    /// The limit it passed.
    pub limit: usize
}

/// Whether trying the same thing again could work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Retryability
{
    /// Retrying changes nothing; something has to change first.
    No,
    /// Retrying now is reasonable.
    Immediate,
    /// Retrying is reasonable after waiting.
    AfterSeconds
    {
        /// How long to wait.
        seconds: u32
    }
}

/// What an upstream said, kept beside trdr's own code.
///
/// This is the status line and the upstream's own short code, never a header or
/// a body. It exists so that a `401` from a broker and an unreachable broker
/// stay two different facts all the way to the screen, and so that a support
/// question can be answered without turning logging back on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct UpstreamStatus
{
    /// The HTTP status, when the exchange got far enough to have one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status: Option<u16>,
    /// The upstream's own short status code, when it publishes one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_code: Option<String>
}

impl UpstreamStatus
{
    /// Records an HTTP status on its own.
    pub const fn http(status: u16) -> Self
    {
        Self {
            http_status: Some(status),
            upstream_code: None
        }
    }

    /// Adds the upstream's own code, if it is short and plain enough to be one.
    ///
    /// The check is not a secret filter — it is the boundary that stops a
    /// response body from being passed off as a status code.
    pub fn with_upstream_code(mut self, code: impl Into<String>)
        -> Result<Self, UpstreamCodeError>
    {
        let code = code.into();

        if code.is_empty() || code.len() > MAX_UPSTREAM_CODE_BYTES
        {
            return Err(UpstreamCodeError::Length { found: code.len() });
        }

        if !code
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
        {
            return Err(UpstreamCodeError::Charset);
        }

        self.upstream_code = Some(code);
        Ok(self)
    }
}

/// An upstream status code was not shaped like a status code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UpstreamCodeError
{
    /// Empty, or longer than [`MAX_UPSTREAM_CODE_BYTES`].
    #[error("expected 1 to {MAX_UPSTREAM_CODE_BYTES} bytes, found {found}")]
    Length
    {
        /// How many bytes arrived.
        found: usize
    },
    /// Held something other than letters, digits, `-`, `_`, and `.`.
    #[error("contains a character that is not a letter, digit, '-', '_', or '.'")]
    Charset
}

/// The group a code belongs to (section 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorFamily
{
    /// Credentials.
    Auth,
    /// An external source.
    Upstream,
    /// An ingest bundle.
    Bundle,
    /// The data itself.
    Data,
    /// A strategy specification.
    Strategy,
    /// The local database.
    Db,
    /// A backup package.
    Backup,
    /// The running app and the protocol to it.
    App,
    /// A human approval.
    Approval,
    /// The agent terminal.
    Terminal
}

impl ErrorFamily
{
    /// The prefix every code in this family carries on the wire.
    pub const fn as_str(self) -> &'static str
    {
        match self
        {
            Self::Auth => "AUTH",
            Self::Upstream => "UPSTREAM",
            Self::Bundle => "BUNDLE",
            Self::Data => "DATA",
            Self::Strategy => "STRATEGY",
            Self::Db => "DB",
            Self::Backup => "BACKUP",
            Self::App => "APP",
            Self::Approval => "APPROVAL",
            Self::Terminal => "TERMINAL"
        }
    }
}

/// Every failure trdr can report.
///
/// The set is closed and comes from section 12. A new kind of failure gets a new
/// variant here rather than a new sentence somewhere, and the exhaustive match in
/// [`ErrorCode::family`] means adding one is a compile error until it has been
/// placed in a family.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode
{
    /// No credential is stored for this source.
    AuthMissing,
    /// The stored credential was refused by the source.
    ///
    /// Distinct from [`ErrorCode::UpstreamUnavailable`] on purpose: this one is
    /// the user's to fix.
    AuthInvalid,
    /// The source is refusing further calls for now.
    AuthRateLimited,
    /// The source could not be reached, or refused to serve.
    UpstreamUnavailable,
    /// The source did not answer in time.
    UpstreamTimeout,
    /// The source answered with something trdr cannot read.
    UpstreamMalformed,
    /// A bundle did not match its declared schema.
    BundleSchema,
    /// A bundle file did not match its recorded hash.
    BundleHash,
    /// A bundle was larger than the limits allow.
    BundleSize,
    /// A bundle record contradicts one already stored.
    BundleConflict,
    /// The data needed for this does not cover the range asked for.
    DataIncomplete,
    /// The data is of a kind this version does not support.
    DataUnsupported,
    /// A value would have been used before it could have been known.
    DataFutureLeak,
    /// Stored data failed its own consistency check.
    DataIntegrity,
    /// A strategy file could not be read as a strategy.
    StrategySyntax,
    /// A strategy asked for a rule this engine does not implement.
    StrategyUnsupportedRule,
    /// A strategy named a period that cannot be run.
    StrategyInvalidPeriod,
    /// The database is held by someone else right now.
    DbBusy,
    /// A schema migration could not be applied.
    DbMigration,
    /// The database failed its integrity check.
    DbIntegrity,
    /// There is no room left on the disk.
    DbDiskFull,
    /// A backup package could not be authenticated or decrypted.
    BackupAuthentication,
    /// A backup package failed a checksum.
    BackupChecksum,
    /// A backup package is from a version this build cannot read.
    BackupUnsupportedVersion,
    /// A backup package is missing part of itself.
    BackupIncomplete,
    /// The app is not running, so this cannot be done.
    AppNotRunning,
    /// The message did not fit the protocol.
    AppProtocolVersion,
    /// The caller is not allowed to ask for this.
    AppPermission,
    /// A person rejected the request.
    ApprovalRejected,
    /// The request was not answered in time.
    ApprovalExpired,
    /// What was approved is no longer what would happen.
    ApprovalStale,
    /// The requester went away before the answer came.
    ApprovalDisconnected,
    /// The agent executable is not where it was expected.
    TerminalExecutableMissing,
    /// The agent process could not be started.
    TerminalSpawn,
    /// The agent process ended.
    TerminalExited
}

impl ErrorCode
{
    /// Every code, in the order section 12 lists them.
    pub const ALL: &'static [ErrorCode] = &[
        Self::AuthMissing,
        Self::AuthInvalid,
        Self::AuthRateLimited,
        Self::UpstreamUnavailable,
        Self::UpstreamTimeout,
        Self::UpstreamMalformed,
        Self::BundleSchema,
        Self::BundleHash,
        Self::BundleSize,
        Self::BundleConflict,
        Self::DataIncomplete,
        Self::DataUnsupported,
        Self::DataFutureLeak,
        Self::DataIntegrity,
        Self::StrategySyntax,
        Self::StrategyUnsupportedRule,
        Self::StrategyInvalidPeriod,
        Self::DbBusy,
        Self::DbMigration,
        Self::DbIntegrity,
        Self::DbDiskFull,
        Self::BackupAuthentication,
        Self::BackupChecksum,
        Self::BackupUnsupportedVersion,
        Self::BackupIncomplete,
        Self::AppNotRunning,
        Self::AppProtocolVersion,
        Self::AppPermission,
        Self::ApprovalRejected,
        Self::ApprovalExpired,
        Self::ApprovalStale,
        Self::ApprovalDisconnected,
        Self::TerminalExecutableMissing,
        Self::TerminalSpawn,
        Self::TerminalExited
    ];

    /// The family this code belongs to.
    pub const fn family(self) -> ErrorFamily
    {
        match self
        {
            Self::AuthMissing | Self::AuthInvalid | Self::AuthRateLimited => ErrorFamily::Auth,
            Self::UpstreamUnavailable | Self::UpstreamTimeout | Self::UpstreamMalformed =>
            {
                ErrorFamily::Upstream
            }
            Self::BundleSchema | Self::BundleHash | Self::BundleSize | Self::BundleConflict =>
            {
                ErrorFamily::Bundle
            }
            Self::DataIncomplete
            | Self::DataUnsupported
            | Self::DataFutureLeak
            | Self::DataIntegrity => ErrorFamily::Data,
            Self::StrategySyntax | Self::StrategyUnsupportedRule | Self::StrategyInvalidPeriod =>
            {
                ErrorFamily::Strategy
            }
            Self::DbBusy | Self::DbMigration | Self::DbIntegrity | Self::DbDiskFull =>
            {
                ErrorFamily::Db
            }
            Self::BackupAuthentication
            | Self::BackupChecksum
            | Self::BackupUnsupportedVersion
            | Self::BackupIncomplete => ErrorFamily::Backup,
            Self::AppNotRunning | Self::AppProtocolVersion | Self::AppPermission =>
            {
                ErrorFamily::App
            }
            Self::ApprovalRejected
            | Self::ApprovalExpired
            | Self::ApprovalStale
            | Self::ApprovalDisconnected => ErrorFamily::Approval,
            Self::TerminalExecutableMissing | Self::TerminalSpawn | Self::TerminalExited =>
            {
                ErrorFamily::Terminal
            }
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn every_code_serialises_to_its_own_string()
    {
        let mut seen = std::collections::BTreeSet::new();

        for code in ErrorCode::ALL
        {
            let wire = serde_json::to_string(code).unwrap();
            assert!(
                seen.insert(wire.clone()),
                "two codes share the wire form {wire}"
            );
        }

        assert_eq!(seen.len(), ErrorCode::ALL.len());
    }

    #[test]
    fn a_code_carries_its_family_as_a_prefix()
    {
        for code in ErrorCode::ALL
        {
            let wire = serde_json::to_string(code).unwrap();
            let wire = wire.trim_matches('"');
            let prefix = format!("{}_", code.family().as_str());
            assert!(
                wire.starts_with(&prefix),
                "{wire} is not in family {prefix}"
            );
        }
    }

    #[test]
    fn the_documented_spellings_are_the_ones_on_the_wire()
    {
        for (code, wire) in [
            (ErrorCode::AppNotRunning, "\"APP_NOT_RUNNING\""),
            (ErrorCode::AuthRateLimited, "\"AUTH_RATE_LIMITED\""),
            (ErrorCode::DataFutureLeak, "\"DATA_FUTURE_LEAK\""),
            (ErrorCode::ApprovalStale, "\"APPROVAL_STALE\""),
            (
                ErrorCode::BackupUnsupportedVersion,
                "\"BACKUP_UNSUPPORTED_VERSION\""
            ),
            (
                ErrorCode::TerminalExecutableMissing,
                "\"TERMINAL_EXECUTABLE_MISSING\""
            )
        ]
        {
            assert_eq!(serde_json::to_string(&code).unwrap(), wire);
            assert_eq!(serde_json::from_str::<ErrorCode>(wire).unwrap(), code);
        }
    }

    #[test]
    fn a_rejected_key_and_an_unreachable_source_stay_apart()
    {
        // The failure this test exists for: a client that turns every non-2xx
        // reply into one error makes these two the same event.
        let bad_key = ErrorEnvelope::new(ErrorCode::AuthInvalid).with_upstream(
            UpstreamStatus::http(401)
                .with_upstream_code("EGW00121")
                .unwrap()
        );
        let source_down = ErrorEnvelope::new(ErrorCode::UpstreamUnavailable)
            .with_upstream(UpstreamStatus::http(503))
            .with_retryability(Retryability::AfterSeconds { seconds: 30 });

        assert_ne!(bad_key.code, source_down.code);
        assert_ne!(bad_key.retryability, source_down.retryability);
        assert_eq!(bad_key.upstream.unwrap().http_status, Some(401));
        assert_eq!(source_down.upstream.unwrap().upstream_code, None);
    }

    #[test]
    fn an_envelope_round_trips()
    {
        let envelope = ErrorEnvelope::new(ErrorCode::AuthRateLimited)
            .with_request("01KZNNR5X818P3J6ENYKSADP8W".parse().unwrap())
            .with_param("collector", ErrorParam::text("trdr.kis").unwrap())
            .with_param("remaining", ErrorParam::Integer(0))
            .with_retryability(Retryability::AfterSeconds { seconds: 60 })
            .with_cause_chain("01KZNP0GQ3X8ARK9DQ489Z7WJ8".parse().unwrap())
            .with_upstream(UpstreamStatus::http(429));

        let json = serde_json::to_string(&envelope).unwrap();
        assert_eq!(
            serde_json::from_str::<ErrorEnvelope>(&json).unwrap(),
            envelope
        );
        assert!(json.contains("\"v\":1"));
        assert!(json.contains("\"code\":\"AUTH_RATE_LIMITED\""));
    }

    #[test]
    fn an_empty_envelope_leaves_out_what_it_does_not_have()
    {
        let json = serde_json::to_string(&ErrorEnvelope::new(ErrorCode::DbBusy)).unwrap();
        assert_eq!(
            json,
            "{\"v\":1,\"code\":\"DB_BUSY\",\"retryability\":{\"kind\":\"no\"}}"
        );
    }

    #[test]
    fn an_envelope_refuses_a_field_it_does_not_know()
    {
        let json =
            "{\"v\":1,\"code\":\"DB_BUSY\",\"retryability\":{\"kind\":\"no\"},\"message\":\"x\"}";
        assert!(serde_json::from_str::<ErrorEnvelope>(json).is_err());
    }

    #[test]
    fn an_envelope_refuses_a_version_it_does_not_speak()
    {
        let json = "{\"v\":2,\"code\":\"DB_BUSY\",\"retryability\":{\"kind\":\"no\"}}";
        assert!(serde_json::from_str::<ErrorEnvelope>(json).is_err());
    }

    #[test]
    fn a_payload_sized_parameter_is_refused()
    {
        let body = "x".repeat(MAX_PARAM_TEXT_BYTES + 1);
        assert_eq!(
            ErrorParam::text(body),
            Err(ParamTooLong {
                found: MAX_PARAM_TEXT_BYTES + 1,
                limit: MAX_PARAM_TEXT_BYTES
            })
        );
        assert!(ErrorParam::text("x".repeat(MAX_PARAM_TEXT_BYTES)).is_ok());
    }

    #[test]
    fn an_upstream_code_that_is_really_a_body_is_refused()
    {
        let status = UpstreamStatus::http(500);
        assert_eq!(
            status.clone().with_upstream_code("{\"msg\":\"boom\"}"),
            Err(UpstreamCodeError::Charset)
        );
        assert_eq!(
            status
                .clone()
                .with_upstream_code("x".repeat(MAX_UPSTREAM_CODE_BYTES + 1)),
            Err(UpstreamCodeError::Length {
                found: MAX_UPSTREAM_CODE_BYTES + 1
            })
        );
        assert_eq!(
            status.with_upstream_code(""),
            Err(UpstreamCodeError::Length { found: 0 })
        );
    }
}
