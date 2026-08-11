//! What the `trdr` CLI says to a running app, and what comes back (section 9.2).
//!
//! The protocol is one JSON object per line over a Unix socket:
//!
//! ```json
//! {"v":1,"id":"...","method":"ui.open","params":{"resource":"strategy","id":"low-vol-v1"}}
//! {"v":1,"id":"...","ok":true,"result":{"status":"opened"}}
//! ```
//!
//! Section 9.2 requires the version and the request id on every frame and
//! requires an unknown method, an unknown field, or an unknown version to be
//! refused rather than ignored. Refusing quietly is the failure mode that
//! matters: a field that gets dropped is a request that did something other than
//! what was asked. So parsing here is deliberately strict and deliberately not a
//! plain `serde_json::from_str` — [`SocketRequest::from_json_line`] is the only
//! way in, it checks the version before anything else, and each refusal is a
//! [`FrameError`] variant rather than a message.
//!
//! These types are Rust on both ends and never cross into TypeScript, which is
//! why — unlike [`crate::ui`] — they carry no `specta` derives. The one thing
//! they share with the WebView side is [`ErrorEnvelope`].

use crate::envelope::EnvelopeVersion;
use crate::error::{ErrorCode, ErrorEnvelope, ErrorParam};
use crate::id::{IdParseError, RequestId, ResourceId};
use crate::query::{AccountSummary, DataCoverage, DataOrigin, Holding};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The most a single frame may weigh.
///
/// A line arrives from another process, so it needs a ceiling somewhere. One
/// mebibyte is far above any request in the initial method list.
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

/// The longest an unknown method name is echoed back in an error.
const MAX_ECHOED_METHOD_BYTES: usize = 64;

/// A frame could not be taken at face value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FrameError
{
    /// The line was longer than [`MAX_FRAME_BYTES`].
    #[error("frame is {found} bytes, the limit is {limit}")]
    TooLarge
    {
        /// How long the line was.
        found: usize,
        /// The limit it passed.
        limit: usize
    },
    /// The line was not JSON, was not an object, or held a field this build does
    /// not know.
    ///
    /// Carries a position rather than the text, so that nothing a caller sent
    /// gets copied into an error and from there into a log.
    #[error("frame is malformed at line {line}, column {column}")]
    Malformed
    {
        /// Line the parser stopped at.
        line: usize,
        /// Column the parser stopped at.
        column: usize
    },
    /// The frame named a protocol version this build does not speak.
    #[error("unsupported protocol version {found}, this build speaks {supported}")]
    UnsupportedVersion
    {
        /// The version the frame claimed.
        found: u16,
        /// The version this build speaks.
        supported: u16
    },
    /// The request id was not a ULID.
    #[error("request id is not usable: {0}")]
    InvalidRequestId(#[from] IdParseError),
    /// The frame named a method this build does not have.
    #[error("unknown method")]
    UnknownMethod
    {
        /// The name that was asked for, cut to a length safe to keep.
        method: String
    },
    /// The method exists but its parameters did not fit.
    #[error("parameters do not fit method {method}")]
    InvalidParams
    {
        /// The method whose parameters did not fit.
        method: &'static str
    },
    /// A response claimed success and carried an error, or the other way round.
    #[error("response says ok={ok} but carries the other half")]
    InconsistentOutcome
    {
        /// What the frame claimed.
        ok: bool
    }
}

impl FrameError
{
    /// The same refusal as the envelope that goes back over the socket.
    ///
    /// Every frame failure is a protocol failure, and section 12 keeps the `APP`
    /// family to three codes, so the code is the same for all of them and the
    /// parameters say which one it was. The distinction stays available to code
    /// through the variants of this type.
    pub fn to_envelope(&self, id: Option<RequestId>) -> ErrorEnvelope
    {
        let mut envelope = ErrorEnvelope::new(ErrorCode::AppProtocolVersion)
            .with_param("reason", ErrorParam::literal(self.reason()));

        if let Some(id) = id
        {
            envelope = envelope.with_request(id);
        }

        match self
        {
            Self::TooLarge { found, limit } => envelope
                .with_param("found", ErrorParam::Integer(*found as i64))
                .with_param("limit", ErrorParam::Integer(*limit as i64)),
            Self::UnsupportedVersion { found, supported } => envelope
                .with_param("found", ErrorParam::Integer(i64::from(*found)))
                .with_param("supported", ErrorParam::Integer(i64::from(*supported))),
            Self::UnknownMethod { method } => match ErrorParam::text(method)
            {
                Ok(param) => envelope.with_param("method", param),
                Err(_) => envelope
            },
            Self::InvalidParams { method } =>
            {
                envelope.with_param("method", ErrorParam::literal(method))
            }
            Self::Malformed { .. }
            | Self::InvalidRequestId(_)
            | Self::InconsistentOutcome { .. } => envelope
        }
    }

    /// A fixed word naming which refusal this is.
    const fn reason(&self) -> &'static str
    {
        match self
        {
            Self::TooLarge { .. } => "frame_too_large",
            Self::Malformed { .. } => "malformed_frame",
            Self::UnsupportedVersion { .. } => "unsupported_version",
            Self::InvalidRequestId(_) => "invalid_request_id",
            Self::UnknownMethod { .. } => "unknown_method",
            Self::InvalidParams { .. } => "invalid_params",
            Self::InconsistentOutcome { .. } => "inconsistent_outcome"
        }
    }
}

impl From<serde_json::Error> for FrameError
{
    fn from(error: serde_json::Error) -> Self
    {
        Self::Malformed {
            line: error.line(),
            column: error.column()
        }
    }
}

/// What a socket method does, so that routing does not have to read the name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketMethodKind
{
    /// Answers from state that already exists and writes nothing.
    Read,
    /// Asks the running app to show something. Only a running app can.
    UiRequest,
    /// Changes durable state, and through this socket the app is what changes
    /// it — the CLI never writes behind a running app's back (section 6).
    Mutation
}

/// Something the app can bring to the front.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiResource
{
    /// A registered strategy.
    Strategy,
    /// A finished backtest.
    Backtest
}

/// Which thing to open (the example in section 9.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct UiOpenParams
{
    /// What kind of thing.
    pub resource: UiResource,
    /// Which one.
    pub id: ResourceId
}

/// An ingest bundle on disk.
///
/// A path, not a handle: the CLI is trusted to name a file (section 2), whereas
/// the WebView is not. The host still runs the path, symlink, and size checks in
/// section 8.2 before reading anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BundlePathParams
{
    /// Where the bundle directory is.
    pub bundle_path: PathBuf
}

/// A backup package on disk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PackagePathParams
{
    /// Where the package is.
    pub package_path: PathBuf
}

/// Where a new backup should be written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DestinationPathParams
{
    /// Where to write it.
    pub destination_path: PathBuf
}

/// A user's strategy file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StrategyPathParams
{
    /// Where the `.trdr.yaml` file is.
    pub strategy_path: PathBuf
}

/// What `app.status` answers with.
///
/// Section 9.2 calls for a bounded read result with no secret and no raw account
/// payload in it, and section 6 says one process holds the writer lease, so this
/// is the running app naming itself, the state it has open, and whether it is the
/// writer. Two paths are in it because "which workspace is this app on" is the
/// question a person asks when two copies are installed; a path is not a secret,
/// and the socket only ever answers the uid that owns it.
///
/// It carries no `specta` derives for the same reason nothing else in this file
/// does: both ends are Rust.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AppStatusResult
{
    /// The version of the running app.
    pub app_version: String,
    /// The process id of the running app.
    pub pid: u32,
    /// Whether that process holds the single writer lease (section 6).
    pub holds_writer_lease: bool,
    /// The product root it is running against.
    pub product_root: PathBuf,
    /// The workspace it has open.
    pub workspace_path: PathBuf,
    /// The database schema version in that workspace.
    pub schema_version: i64
}

/// What `account.inspect` answers with.
///
/// The same values the Today screen renders, and deliberately the same types:
/// milestone M2 requires the UI and the CLI to read one domain object rather
/// than two that are kept in step by hand, and reusing
/// [`crate::query::AccountSummary`] is what makes that true by construction
/// instead of by a test comparing two shapes.
///
/// Bounded, as section 9.2 requires of a read: the totals and the positions, no
/// token, no account number, and no raw broker payload. There is nowhere in
/// [`crate::query::AccountSummary`] to put one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AccountInspectResult
{
    /// Whether these numbers are real, carried for the same reason a screen
    /// carries it: a person reading terminal output is owed the same statement
    /// as a person reading a window.
    pub origin: DataOrigin,
    /// The totals.
    pub account: AccountSummary,
    /// The positions behind them.
    pub holdings: Vec<Holding>
}

/// What `data.coverage` answers with.
///
/// The same [`crate::query::DataCoverage`] the Lab draft screen shows, for the
/// same reason [`AccountInspectResult`] reuses the account summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct DataCoverageResult
{
    /// Whether this describes real collected data.
    pub origin: DataOrigin,
    /// What the requested period needs, and how much of it is present.
    pub coverage: Vec<DataCoverage>
}

/// Every method the CLI can call on a running app.
///
/// Methods whose parameters are not yet fixed are marked. The track that
/// implements one adds its parameters with it; nothing is invented here that the
/// design does not already pin down.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum SocketMethod
{
    /// Whether an app is there, and what it is holding.
    #[serde(rename = "app.status")]
    AppStatus,
    /// Bring something up on screen.
    #[serde(rename = "ui.open")]
    UiOpen(UiOpenParams),
    /// Bring the app itself to the front.
    #[serde(rename = "ui.focus")]
    UiFocus,
    /// The latest normalised account snapshot. Never the raw response, never a
    /// token, never an account number (sections 5.2 and 9.2).
    #[serde(rename = "account.inspect")]
    AccountInspect,
    /// What data is present for a range. Parameters land with the data track.
    #[serde(rename = "data.coverage")]
    DataCoverage,
    /// Validate and commit a bundle.
    #[serde(rename = "ingest.request")]
    IngestRequest(BundlePathParams),
    /// Run a backtest. Parameters land with the engine track.
    #[serde(rename = "backtest.run")]
    BacktestRun,
    /// Look at a finished backtest. Parameters land with the engine track.
    #[serde(rename = "backtest.inspect")]
    BacktestInspect,
    /// Ask for a portable backup.
    #[serde(rename = "backup.create.request")]
    BackupCreateRequest(DestinationPathParams),
    /// Check a package without restoring it.
    #[serde(rename = "backup.verify")]
    BackupVerify(PackagePathParams),
    /// Ask to restore a package into a new workspace.
    #[serde(rename = "workspace.restore.request")]
    WorkspaceRestoreRequest(PackagePathParams),
    /// Read a strategy file and report what it means.
    #[serde(rename = "strategy.inspect")]
    StrategyInspect(StrategyPathParams),
    /// Ask a person to approve registering a strategy (section 9.3).
    ///
    /// There is no method that registers without the round trip, and there is no
    /// method that bypasses the sheet.
    #[serde(rename = "strategy.register.request")]
    StrategyRegisterRequest(StrategyPathParams)
}

impl SocketMethod
{
    /// Every method name, in the order section 9.2 lists them.
    pub const NAMES: &'static [&'static str] = &[
        "app.status",
        "ui.open",
        "ui.focus",
        "account.inspect",
        "data.coverage",
        "ingest.request",
        "backtest.run",
        "backtest.inspect",
        "backup.create.request",
        "backup.verify",
        "workspace.restore.request",
        "strategy.inspect",
        "strategy.register.request"
    ];

    /// Whether a name is one this build has.
    pub fn is_known(name: &str) -> bool
    {
        Self::NAMES.contains(&name)
    }

    /// The name this method goes by on the wire.
    pub const fn name(&self) -> &'static str
    {
        match self
        {
            Self::AppStatus => "app.status",
            Self::UiOpen(_) => "ui.open",
            Self::UiFocus => "ui.focus",
            Self::AccountInspect => "account.inspect",
            Self::DataCoverage => "data.coverage",
            Self::IngestRequest(_) => "ingest.request",
            Self::BacktestRun => "backtest.run",
            Self::BacktestInspect => "backtest.inspect",
            Self::BackupCreateRequest(_) => "backup.create.request",
            Self::BackupVerify(_) => "backup.verify",
            Self::WorkspaceRestoreRequest(_) => "workspace.restore.request",
            Self::StrategyInspect(_) => "strategy.inspect",
            Self::StrategyRegisterRequest(_) => "strategy.register.request"
        }
    }

    /// What this method does, for whoever routes it.
    pub const fn kind(&self) -> SocketMethodKind
    {
        match self
        {
            Self::AppStatus
            | Self::AccountInspect
            | Self::DataCoverage
            | Self::BacktestInspect
            | Self::BackupVerify(_)
            | Self::StrategyInspect(_) => SocketMethodKind::Read,
            Self::UiOpen(_) | Self::UiFocus => SocketMethodKind::UiRequest,
            Self::IngestRequest(_)
            | Self::BacktestRun
            | Self::BackupCreateRequest(_)
            | Self::WorkspaceRestoreRequest(_)
            | Self::StrategyRegisterRequest(_) => SocketMethodKind::Mutation
        }
    }
}

/// One request line.
///
/// There is no `Deserialize` for this type on purpose. Everything comes in
/// through [`SocketRequest::from_json_line`], so the strict checks cannot be
/// skipped by reaching for `serde_json` directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocketRequest
{
    /// Protocol version.
    pub v: EnvelopeVersion,
    /// Correlates this request with its response.
    pub id: RequestId,
    /// What is being asked for.
    pub method: SocketMethod
}

/// The exact set of fields a request line may have.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestWire
{
    v: u16,
    id: String,
    method: String,
    #[serde(default)]
    params: Option<serde_json::Value>
}

impl SocketRequest
{
    /// A request at the current protocol version.
    pub fn new(id: RequestId, method: SocketMethod) -> Self
    {
        Self {
            v: EnvelopeVersion::CURRENT,
            id,
            method
        }
    }

    /// Writes the request as one line, newline included.
    pub fn to_json_line(&self) -> Result<String, serde_json::Error>
    {
        #[derive(Serialize)]
        struct Out<'a>
        {
            v: EnvelopeVersion,
            id: &'a RequestId,
            #[serde(flatten)]
            method: &'a SocketMethod
        }

        let mut line = serde_json::to_string(&Out {
            v: self.v,
            id: &self.id,
            method: &self.method
        })?;
        line.push('\n');
        Ok(line)
    }

    /// Reads one line, refusing everything section 9.2 says to refuse.
    ///
    /// The version is checked before the method, so a peer speaking a later
    /// protocol is told that and not that its method is unknown.
    pub fn from_json_line(line: &str) -> Result<Self, FrameError>
    {
        let wire: RequestWire = parse_line(line)?;
        let v = check_version(wire.v)?;
        let id: RequestId = wire.id.parse()?;
        let method = parse_method(&wire.method, wire.params)?;

        Ok(Self { v, id, method })
    }
}

/// One response line.
///
/// Generic over what a successful result carries. Each method's result type
/// arrives with the method; until then the default is raw JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct SocketResponse<R = serde_json::Value>
{
    /// Protocol version.
    pub v: EnvelopeVersion,
    /// The request this answers.
    pub id: RequestId,
    /// How it went.
    pub outcome: SocketOutcome<R>
}

/// Either a result or an error envelope, never both and never neither.
#[derive(Debug, Clone, PartialEq)]
pub enum SocketOutcome<R>
{
    /// It worked, and this is what came back.
    Ok(R),
    /// It did not.
    Error(ErrorEnvelope)
}

/// The exact set of fields a response line may have.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResponseWire
{
    v: u16,
    id: String,
    ok: bool,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<ErrorEnvelope>
}

impl<R> SocketResponse<R>
{
    /// A successful answer at the current protocol version.
    pub fn ok(id: RequestId, result: R) -> Self
    {
        Self {
            v: EnvelopeVersion::CURRENT,
            id,
            outcome: SocketOutcome::Ok(result)
        }
    }

    /// A failed one.
    pub fn error(id: RequestId, error: ErrorEnvelope) -> Self
    {
        Self {
            v: EnvelopeVersion::CURRENT,
            id,
            outcome: SocketOutcome::Error(error)
        }
    }
}

impl<R: Serialize> SocketResponse<R>
{
    /// Writes the response as one line, newline included.
    pub fn to_json_line(&self) -> Result<String, serde_json::Error>
    {
        #[derive(Serialize)]
        struct Out<'a, R>
        {
            v: EnvelopeVersion,
            id: &'a RequestId,
            ok: bool,
            #[serde(skip_serializing_if = "Option::is_none")]
            result: Option<&'a R>,
            #[serde(skip_serializing_if = "Option::is_none")]
            error: Option<&'a ErrorEnvelope>
        }

        let (ok, result, error) = match &self.outcome
        {
            SocketOutcome::Ok(result) => (true, Some(result), None),
            SocketOutcome::Error(error) => (false, None, Some(error))
        };

        let mut line = serde_json::to_string(&Out {
            v: self.v,
            id: &self.id,
            ok,
            result,
            error
        })?;
        line.push('\n');
        Ok(line)
    }
}

impl<R: serde::de::DeserializeOwned> SocketResponse<R>
{
    /// Reads one line, refusing a frame that claims one outcome and carries the
    /// other.
    pub fn from_json_line(line: &str) -> Result<Self, FrameError>
    {
        let wire: ResponseWire = parse_line(line)?;
        let v = check_version(wire.v)?;
        let id: RequestId = wire.id.parse()?;

        let outcome = match (wire.ok, wire.result, wire.error)
        {
            (true, Some(result), None) => SocketOutcome::Ok(serde_json::from_value(result)?),
            (false, None, Some(error)) => SocketOutcome::Error(error),
            (ok, _, _) => return Err(FrameError::InconsistentOutcome { ok })
        };

        Ok(Self { v, id, outcome })
    }
}

/// Reads one line into its wire shape, with the size limit applied first.
fn parse_line<T: serde::de::DeserializeOwned>(line: &str) -> Result<T, FrameError>
{
    if line.len() > MAX_FRAME_BYTES
    {
        return Err(FrameError::TooLarge {
            found: line.len(),
            limit: MAX_FRAME_BYTES
        });
    }

    Ok(serde_json::from_str(line.trim_end_matches(['\n', '\r']))?)
}

/// Turns the raw version number into a checked one.
fn check_version(raw: u16) -> Result<EnvelopeVersion, FrameError>
{
    EnvelopeVersion::supported(raw).map_err(|error| FrameError::UnsupportedVersion {
        found: error.found,
        supported: error.supported
    })
}

/// Puts the method name and its parameters back together, telling a method this
/// build does not have apart from parameters that do not fit it.
fn parse_method(name: &str, params: Option<serde_json::Value>) -> Result<SocketMethod, FrameError>
{
    if !SocketMethod::is_known(name)
    {
        let cut = name
            .char_indices()
            .map(|(i, _)| i)
            .nth(MAX_ECHOED_METHOD_BYTES);

        return Err(FrameError::UnknownMethod {
            method: name[..cut.unwrap_or(name.len())].to_owned()
        });
    }

    let mut fields = serde_json::Map::new();
    fields.insert(
        "method".to_owned(),
        serde_json::Value::String(name.to_owned())
    );

    if let Some(params) = params
    {
        fields.insert("params".to_owned(), params);
    }

    serde_json::from_value(serde_json::Value::Object(fields)).map_err(|_| {
        let known = SocketMethod::NAMES.iter().find(|known| **known == name);
        FrameError::InvalidParams {
            method: known.copied().unwrap_or("unknown")
        }
    })
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::envelope::PROTOCOL_VERSION;

    const REQUEST: &str = "01KZNNR5X818P3J6ENYKSADP8W";

    fn every_method() -> Vec<SocketMethod>
    {
        let package = PackagePathParams {
            package_path: PathBuf::from("/tmp/a.trdrbackup")
        };
        let strategy = StrategyPathParams {
            strategy_path: PathBuf::from("/tmp/a.trdr.yaml")
        };

        vec![
            SocketMethod::AppStatus,
            SocketMethod::UiOpen(UiOpenParams {
                resource: UiResource::Strategy,
                id: ResourceId::parse("low-vol-v1").unwrap()
            }),
            SocketMethod::UiFocus,
            SocketMethod::AccountInspect,
            SocketMethod::DataCoverage,
            SocketMethod::IngestRequest(BundlePathParams {
                bundle_path: PathBuf::from("/tmp/my-bundle")
            }),
            SocketMethod::BacktestRun,
            SocketMethod::BacktestInspect,
            SocketMethod::BackupCreateRequest(DestinationPathParams {
                destination_path: PathBuf::from("/tmp/out")
            }),
            SocketMethod::BackupVerify(package.clone()),
            SocketMethod::WorkspaceRestoreRequest(package),
            SocketMethod::StrategyInspect(strategy.clone()),
            SocketMethod::StrategyRegisterRequest(strategy),
        ]
    }

    #[test]
    fn the_line_in_the_design_document_parses_to_what_it_says()
    {
        let line = "{\"v\":1,\"id\":\"01KZNNR5X818P3J6ENYKSADP8W\",\"method\":\"ui.open\",\
                    \"params\":{\"resource\":\"strategy\",\"id\":\"low-vol-v1\"}}";
        let request = SocketRequest::from_json_line(line).unwrap();

        assert_eq!(request.v, EnvelopeVersion::CURRENT);
        assert_eq!(request.id, REQUEST.parse::<RequestId>().unwrap());
        assert_eq!(
            request.method,
            SocketMethod::UiOpen(UiOpenParams {
                resource: UiResource::Strategy,
                id: ResourceId::parse("low-vol-v1").unwrap()
            })
        );
    }

    #[test]
    fn every_method_round_trips_through_a_line()
    {
        for method in every_method()
        {
            let request = SocketRequest::new(REQUEST.parse().unwrap(), method);
            let line = request.to_json_line().unwrap();

            assert!(line.ends_with('\n'));
            assert_eq!(line.matches('\n').count(), 1);
            assert_eq!(SocketRequest::from_json_line(&line).unwrap(), request);
        }
    }

    #[test]
    fn a_method_name_is_the_one_the_design_fixes()
    {
        let names: Vec<&str> = every_method().iter().map(|m| m.name()).collect();
        assert_eq!(names, SocketMethod::NAMES);

        for method in every_method()
        {
            let value = serde_json::to_value(&method).unwrap();
            assert_eq!(value["method"], method.name());
        }
    }

    #[test]
    fn each_method_is_on_a_known_side_of_the_write_boundary()
    {
        let of_kind = |kind: SocketMethodKind| -> Vec<&'static str> {
            every_method()
                .iter()
                .filter(|m| m.kind() == kind)
                .map(|m| m.name())
                .collect()
        };

        assert_eq!(
            of_kind(SocketMethodKind::Read),
            [
                "app.status",
                "account.inspect",
                "data.coverage",
                "backtest.inspect",
                "backup.verify",
                "strategy.inspect"
            ]
        );
        assert_eq!(
            of_kind(SocketMethodKind::UiRequest),
            ["ui.open", "ui.focus"]
        );
        assert_eq!(
            of_kind(SocketMethodKind::Mutation),
            [
                "ingest.request",
                "backtest.run",
                "backup.create.request",
                "workspace.restore.request",
                "strategy.register.request"
            ]
        );
    }

    #[test]
    fn a_version_this_build_does_not_speak_is_a_typed_refusal()
    {
        let line = format!("{{\"v\":2,\"id\":\"{REQUEST}\",\"method\":\"app.status\"}}");
        assert_eq!(
            SocketRequest::from_json_line(&line),
            Err(FrameError::UnsupportedVersion {
                found: 2,
                supported: PROTOCOL_VERSION
            })
        );
    }

    #[test]
    fn the_version_is_checked_before_the_method()
    {
        // A peer one version ahead will be sending methods this build has never
        // heard of. It should be told about the version, not the method.
        let line = format!("{{\"v\":9,\"id\":\"{REQUEST}\",\"method\":\"quantum.leap\"}}");
        assert!(matches!(
            SocketRequest::from_json_line(&line),
            Err(FrameError::UnsupportedVersion { .. })
        ));
    }

    #[test]
    fn an_unknown_method_is_refused_and_named_back_in_bounded_form()
    {
        let line = format!("{{\"v\":1,\"id\":\"{REQUEST}\",\"method\":\"db.query\"}}");
        assert_eq!(
            SocketRequest::from_json_line(&line),
            Err(FrameError::UnknownMethod {
                method: "db.query".to_owned()
            })
        );

        let long = "x".repeat(4096);
        let line = format!("{{\"v\":1,\"id\":\"{REQUEST}\",\"method\":\"{long}\"}}");

        match SocketRequest::from_json_line(&line)
        {
            Err(FrameError::UnknownMethod { method }) => assert_eq!(method.len(), 64),
            other => panic!("expected an unknown method, got {other:?}")
        }
    }

    #[test]
    fn an_unknown_field_is_refused_rather_than_dropped()
    {
        let line =
            format!("{{\"v\":1,\"id\":\"{REQUEST}\",\"method\":\"app.status\",\"dry_run\":false}}");
        assert!(matches!(
            SocketRequest::from_json_line(&line),
            Err(FrameError::Malformed { .. })
        ));
    }

    #[test]
    fn an_unknown_field_inside_parameters_is_refused_too()
    {
        let line = format!(
            "{{\"v\":1,\"id\":\"{REQUEST}\",\"method\":\"ui.open\",\
             \"params\":{{\"resource\":\"strategy\",\"id\":\"low-vol-v1\",\"force\":true}}}}"
        );
        assert_eq!(
            SocketRequest::from_json_line(&line),
            Err(FrameError::InvalidParams { method: "ui.open" })
        );
    }

    #[test]
    fn missing_parameters_are_refused()
    {
        let line = format!("{{\"v\":1,\"id\":\"{REQUEST}\",\"method\":\"ui.open\"}}");
        assert_eq!(
            SocketRequest::from_json_line(&line),
            Err(FrameError::InvalidParams { method: "ui.open" })
        );
    }

    #[test]
    fn a_method_that_takes_nothing_refuses_parameters()
    {
        let line = format!(
            "{{\"v\":1,\"id\":\"{REQUEST}\",\"method\":\"app.status\",\"params\":{{\"verbose\":true}}}}"
        );
        assert_eq!(
            SocketRequest::from_json_line(&line),
            Err(FrameError::InvalidParams {
                method: "app.status"
            })
        );
    }

    #[test]
    fn a_value_with_a_newline_in_it_still_makes_one_line()
    {
        // The protocol is one object per line, so a path a user chose has to be
        // escaped rather than allowed to end the frame early.
        let request = SocketRequest::new(
            REQUEST.parse().unwrap(),
            SocketMethod::IngestRequest(BundlePathParams {
                bundle_path: PathBuf::from("/tmp/a\nb")
            })
        );
        let line = request.to_json_line().unwrap();

        assert_eq!(line.matches('\n').count(), 1);
        assert!(line.ends_with('\n'));
        assert_eq!(SocketRequest::from_json_line(&line).unwrap(), request);
    }

    #[test]
    fn a_request_id_that_is_not_a_ulid_is_refused()
    {
        let line = "{\"v\":1,\"id\":\"1\",\"method\":\"app.status\"}";
        assert_eq!(
            SocketRequest::from_json_line(line),
            Err(FrameError::InvalidRequestId(IdParseError::Length {
                found: 1
            }))
        );
    }

    #[test]
    fn an_oversized_frame_is_refused_before_it_is_parsed()
    {
        let line = format!(
            "{{\"v\":1,\"id\":\"{REQUEST}\",\"pad\":\"{}\"}}",
            "x".repeat(MAX_FRAME_BYTES)
        );
        assert!(matches!(
            SocketRequest::from_json_line(&line),
            Err(FrameError::TooLarge {
                limit: MAX_FRAME_BYTES,
                ..
            })
        ));
    }

    #[test]
    fn a_malformed_frame_reports_a_position_and_not_its_contents()
    {
        let secret = "sk-do-not-echo-this";
        let line =
            format!("{{\"v\":1,\"id\":\"{REQUEST}\",\"method\":\"app.status\",\"k\":\"{secret}\"");

        match SocketRequest::from_json_line(&line)
        {
            Err(error) => assert!(!format!("{error}").contains(secret)),
            Ok(_) => panic!("a truncated frame should not parse")
        }
    }

    #[test]
    fn a_response_round_trips_either_way()
    {
        let id: RequestId = REQUEST.parse().unwrap();
        let ok: SocketResponse = SocketResponse::ok(id, serde_json::json!({"status": "opened"}));
        let line = ok.to_json_line().unwrap();

        assert_eq!(
            line,
            format!("{{\"v\":1,\"id\":\"{REQUEST}\",\"ok\":true,\"result\":{{\"status\":\"opened\"}}}}\n")
        );
        assert_eq!(
            SocketResponse::<serde_json::Value>::from_json_line(&line).unwrap(),
            ok
        );

        let failed: SocketResponse =
            SocketResponse::error(id, ErrorEnvelope::new(ErrorCode::AppNotRunning));
        let line = failed.to_json_line().unwrap();
        assert_eq!(
            SocketResponse::<serde_json::Value>::from_json_line(&line).unwrap(),
            failed
        );
    }

    #[test]
    fn a_response_that_claims_one_outcome_and_carries_the_other_is_refused()
    {
        for line in [
            format!("{{\"v\":1,\"id\":\"{REQUEST}\",\"ok\":true}}"),
            format!(
                "{{\"v\":1,\"id\":\"{REQUEST}\",\"ok\":true,\"result\":{{}},\
                 \"error\":{{\"v\":1,\"code\":\"DB_BUSY\",\"retryability\":{{\"kind\":\"no\"}}}}}}"
            ),
            format!("{{\"v\":1,\"id\":\"{REQUEST}\",\"ok\":false}}")
        ]
        {
            assert!(matches!(
                SocketResponse::<serde_json::Value>::from_json_line(&line),
                Err(FrameError::InconsistentOutcome { .. })
            ));
        }
    }

    #[test]
    fn an_app_status_result_round_trips_and_refuses_a_field_it_does_not_know()
    {
        let id: RequestId = REQUEST.parse().unwrap();
        let status = AppStatusResult {
            app_version: "0.0.0".to_owned(),
            pid: 4321,
            holds_writer_lease: true,
            product_root: PathBuf::from("/private/tmp/root"),
            workspace_path: PathBuf::from("/private/tmp/root/workspaces/default"),
            schema_version: 1
        };

        let line = SocketResponse::ok(id, status.clone())
            .to_json_line()
            .unwrap();
        let read = SocketResponse::<AppStatusResult>::from_json_line(&line).unwrap();

        assert_eq!(read.outcome, SocketOutcome::Ok(status));

        let extra = line.replace("\"pid\":4321", "\"pid\":4321,\"token\":\"x\"");
        assert!(SocketResponse::<AppStatusResult>::from_json_line(&extra).is_err());
    }

    #[test]
    fn a_typed_result_is_read_as_that_type()
    {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Status
        {
            running: bool
        }

        let id: RequestId = REQUEST.parse().unwrap();
        let response = SocketResponse::ok(id, Status { running: true });
        let line = response.to_json_line().unwrap();

        assert_eq!(
            SocketResponse::<Status>::from_json_line(&line).unwrap(),
            response
        );
        assert!(SocketResponse::<u32>::from_json_line(&line).is_err());
    }

    #[test]
    fn every_refusal_becomes_an_envelope_that_says_which_one_it_was()
    {
        let id: RequestId = REQUEST.parse().unwrap();
        let refusals = [
            FrameError::TooLarge { found: 2, limit: 1 },
            FrameError::Malformed { line: 1, column: 2 },
            FrameError::UnsupportedVersion {
                found: 2,
                supported: 1
            },
            FrameError::InvalidRequestId(IdParseError::Alphabet),
            FrameError::UnknownMethod {
                method: "db.query".to_owned()
            },
            FrameError::InvalidParams { method: "ui.open" },
            FrameError::InconsistentOutcome { ok: true }
        ];

        let mut reasons = std::collections::BTreeSet::new();

        for refusal in refusals
        {
            let envelope = refusal.to_envelope(Some(id));

            assert_eq!(envelope.code, ErrorCode::AppProtocolVersion);
            assert_eq!(envelope.id, Some(id));

            match envelope.params.get("reason")
            {
                Some(ErrorParam::Text(reason)) => assert!(reasons.insert(reason.clone())),
                other => panic!("expected a reason, got {other:?}")
            }
        }

        assert_eq!(reasons.len(), 7);
    }
}
