//! What the React WebView is allowed to ask the Tauri host for (section 9.1).
//!
//! The WebView has no Keychain, no database, no filesystem, and no network. It
//! has this list. Every command below is one the app wrote and registered by
//! name; the generic shell, filesystem, and SQL plugin commands are never
//! exposed, so this enum is the whole surface a screen can reach.
//!
//! Two things are load-bearing here beyond the names:
//!
//! - A screen never passes a path. It passes a [`ScopedPathHandle`], which the
//!   host minted after the user picked something in a native panel. The mapping
//!   from handle to real location stays in the host, so a screen cannot name a
//!   file the user did not choose.
//! - Reading and intending are separated by [`CommandKind`], not by convention.
//!   Section 2 requires the boundary between a read and a durable change to be
//!   fixed per command, and [`UiCommand::kind`] is where it is fixed.
//!
//! Stage 0 defines the shapes. No command is handled here, and several carry no
//! parameters yet — those are marked, and the track that implements a command
//! adds its parameters with it.

use crate::envelope::EnvelopeVersion;
use crate::error::ErrorEnvelope;
use crate::id::{ApprovalRequestId, RequestId, ResourceId, ScopedPathHandle};
use serde::{Deserialize, Serialize};

/// A command from the WebView, with the version and request id every trdr
/// message carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct UiCommandEnvelope
{
    /// Protocol version.
    pub v: EnvelopeVersion,
    /// Correlates this command with its response.
    pub id: RequestId,
    /// What is being asked for.
    pub command: UiCommand
}

impl UiCommandEnvelope
{
    /// An envelope at the current protocol version.
    pub fn new(id: RequestId, command: UiCommand) -> Self
    {
        Self {
            v: EnvelopeVersion::CURRENT,
            id,
            command
        }
    }
}

/// The answer to a [`UiCommandEnvelope`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct UiResponseEnvelope<T>
{
    /// Protocol version.
    pub v: EnvelopeVersion,
    /// The command this answers.
    pub id: RequestId,
    /// How it went.
    pub outcome: UiOutcome<T>
}

impl<T> UiResponseEnvelope<T>
{
    /// A successful answer.
    pub fn ok(id: RequestId, value: T) -> Self
    {
        Self {
            v: EnvelopeVersion::CURRENT,
            id,
            outcome: UiOutcome::Ok(value)
        }
    }

    /// A failed one.
    pub fn error(id: RequestId, error: ErrorEnvelope) -> Self
    {
        Self {
            v: EnvelopeVersion::CURRENT,
            id,
            outcome: UiOutcome::Error(error)
        }
    }
}

/// Either a result or an error envelope, never both and never neither.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum UiOutcome<T>
{
    /// The command succeeded, and this is what it produced.
    Ok(T),
    /// The command failed.
    Error(ErrorEnvelope)
}

/// Whether a command only looks, or intends a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind
{
    /// Returns a query model and changes nothing.
    Read,
    /// Asks for something to happen: a job, a native sheet, a process.
    Intent
}

/// One of the three sources trdr collects from itself.
///
/// User-written collectors are not here. They do not run inside the app and do
/// not have commands; they write a bundle and it goes through ingest (section
/// 8.3). KRX is not here either, and section 1 keeps it that way.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, specta::Type,
)]
pub enum CollectorId
{
    /// Korea Investment & Securities.
    #[serde(rename = "trdr.kis")]
    Kis,
    /// OpenDART, the disclosure service.
    #[serde(rename = "trdr.opendart")]
    OpenDart,
    /// ECOS, the Bank of Korea statistics service.
    #[serde(rename = "trdr.ecos")]
    Ecos
}

/// How a person answered an approval sheet (section 9.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision
{
    /// Let it happen.
    Approve,
    /// Do not.
    Reject
}

/// Names one of the three collectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CollectorParams
{
    /// Which collector.
    pub collector: CollectorId
}

/// Names a strategy the user has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StrategyParams
{
    /// The strategy's user-facing id.
    pub strategy: ResourceId
}

/// Names a location the user picked in a native panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PathHandleParams
{
    /// The handle the host minted for that location.
    pub path: ScopedPathHandle
}

/// Answers one pending approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct ApprovalRespondParams
{
    /// Which request is being answered.
    pub request: ApprovalRequestId,
    /// The answer.
    pub decision: ApprovalDecision
}

/// Keystrokes for the agent terminal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TerminalInputParams
{
    /// Raw bytes, base64 encoded.
    ///
    /// Bytes, not text: section 1 says the agent's output is a byte stream and
    /// nothing on the way through interprets it. The same holds going in.
    pub data_base64: String
}

/// A new terminal size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TerminalResizeParams
{
    /// Columns.
    pub cols: u16,
    /// Rows.
    pub rows: u16
}

/// Every command the WebView can send.
///
/// On the wire a command is `{"method": "...", "params": {...}}`, with `params`
/// left out entirely where a command has none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "method", content = "params", deny_unknown_fields)]
pub enum UiCommand
{
    /// What the app needs before it can show anything.
    #[serde(rename = "bootstrap.get")]
    BootstrapGet,
    /// The Today screen's model.
    #[serde(rename = "today.get")]
    TodayGet,
    /// Every collector and its credential state.
    #[serde(rename = "collectors.list")]
    CollectorsList,
    /// The current strategy draft as the Lab sees it.
    ///
    /// Parameters land with the Lab track.
    #[serde(rename = "lab.draft.get")]
    LabDraftGet,
    /// A finished backtest.
    ///
    /// Parameters land with the Lab track.
    #[serde(rename = "backtest.get")]
    BacktestGet,
    /// Every registered strategy.
    #[serde(rename = "strategies.list")]
    StrategiesList,
    /// One registered strategy in detail.
    #[serde(rename = "strategy.get")]
    StrategyGet(StrategyParams),
    /// Running and recent jobs.
    #[serde(rename = "jobs.get")]
    JobsGet,
    /// Backups this workspace knows about.
    #[serde(rename = "backup.list")]
    BackupList,
    /// Raise the native sheet for a collector's credentials.
    ///
    /// The secret goes from a secure field straight to the Keychain. The WebView
    /// only ever learns the resulting state (section 10).
    #[serde(rename = "collector.credential.open_native_sheet")]
    CollectorCredentialOpenNativeSheet(CollectorParams),
    /// Try the stored credential against the source.
    #[serde(rename = "collector.test")]
    CollectorTest(CollectorParams),
    /// Collect from the source.
    #[serde(rename = "collector.collect")]
    CollectorCollect(CollectorParams),
    /// Pick a workspace.
    ///
    /// Takes nothing: it opens the native panel that produces the handle.
    #[serde(rename = "workspace.choose")]
    WorkspaceChoose,
    /// Validate a bundle and report what committing it would do, changing
    /// nothing (section 8.2).
    #[serde(rename = "ingest.dry_run")]
    IngestDryRun(PathHandleParams),
    /// Commit a bundle in one transaction.
    #[serde(rename = "ingest.commit")]
    IngestCommit(PathHandleParams),
    /// Run a backtest.
    ///
    /// Parameters land with the Lab track.
    #[serde(rename = "backtest.run")]
    BacktestRun,
    /// Write a portable backup to the chosen place.
    #[serde(rename = "backup.create")]
    BackupCreate(PathHandleParams),
    /// Check a backup package without restoring it.
    #[serde(rename = "backup.verify")]
    BackupVerify(PathHandleParams),
    /// Raise the native sheet that walks through restoring a backup.
    #[serde(rename = "workspace.restore.open_native_sheet")]
    WorkspaceRestoreOpenNativeSheet(PathHandleParams),
    /// Answer a pending approval.
    #[serde(rename = "approval.respond")]
    ApprovalRespond(ApprovalRespondParams),
    /// Start the agent terminal.
    ///
    /// Which executable to run is a setting, not a parameter a screen supplies.
    #[serde(rename = "terminal.start")]
    TerminalStart,
    /// Send bytes to the agent.
    #[serde(rename = "terminal.input")]
    TerminalInput(TerminalInputParams),
    /// Tell the agent the terminal changed size.
    #[serde(rename = "terminal.resize")]
    TerminalResize(TerminalResizeParams),
    /// Stop the agent and start it again.
    #[serde(rename = "terminal.restart")]
    TerminalRestart
}

impl UiCommand
{
    /// Whether this command reads or intends a change.
    ///
    /// The match is exhaustive, so a command added later cannot be registered
    /// without someone deciding which side of the boundary it is on.
    pub const fn kind(&self) -> CommandKind
    {
        match self
        {
            Self::BootstrapGet
            | Self::TodayGet
            | Self::CollectorsList
            | Self::LabDraftGet
            | Self::BacktestGet
            | Self::StrategiesList
            | Self::StrategyGet(_)
            | Self::JobsGet
            | Self::BackupList => CommandKind::Read,
            Self::CollectorCredentialOpenNativeSheet(_)
            | Self::CollectorTest(_)
            | Self::CollectorCollect(_)
            | Self::WorkspaceChoose
            | Self::IngestDryRun(_)
            | Self::IngestCommit(_)
            | Self::BacktestRun
            | Self::BackupCreate(_)
            | Self::BackupVerify(_)
            | Self::WorkspaceRestoreOpenNativeSheet(_)
            | Self::ApprovalRespond(_)
            | Self::TerminalStart
            | Self::TerminalInput(_)
            | Self::TerminalResize(_)
            | Self::TerminalRestart => CommandKind::Intent
        }
    }

    /// The name this command goes by on the wire and in the capability file.
    pub const fn method(&self) -> &'static str
    {
        match self
        {
            Self::BootstrapGet => "bootstrap.get",
            Self::TodayGet => "today.get",
            Self::CollectorsList => "collectors.list",
            Self::LabDraftGet => "lab.draft.get",
            Self::BacktestGet => "backtest.get",
            Self::StrategiesList => "strategies.list",
            Self::StrategyGet(_) => "strategy.get",
            Self::JobsGet => "jobs.get",
            Self::BackupList => "backup.list",
            Self::CollectorCredentialOpenNativeSheet(_) => "collector.credential.open_native_sheet",
            Self::CollectorTest(_) => "collector.test",
            Self::CollectorCollect(_) => "collector.collect",
            Self::WorkspaceChoose => "workspace.choose",
            Self::IngestDryRun(_) => "ingest.dry_run",
            Self::IngestCommit(_) => "ingest.commit",
            Self::BacktestRun => "backtest.run",
            Self::BackupCreate(_) => "backup.create",
            Self::BackupVerify(_) => "backup.verify",
            Self::WorkspaceRestoreOpenNativeSheet(_) => "workspace.restore.open_native_sheet",
            Self::ApprovalRespond(_) => "approval.respond",
            Self::TerminalStart => "terminal.start",
            Self::TerminalInput(_) => "terminal.input",
            Self::TerminalResize(_) => "terminal.resize",
            Self::TerminalRestart => "terminal.restart"
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    const REQUEST: &str = "01KZNNR5X818P3J6ENYKSADP8W";
    const HANDLE: &str = "01KZNP0GQ3X8ARK9DQ489Z7WJ8";

    /// One of every command, so that the tests below cover the whole surface
    /// rather than a sample of it.
    fn every_command() -> Vec<UiCommand>
    {
        let path = PathHandleParams {
            path: HANDLE.parse().unwrap()
        };
        let collector = CollectorParams {
            collector: CollectorId::Kis
        };

        vec![
            UiCommand::BootstrapGet,
            UiCommand::TodayGet,
            UiCommand::CollectorsList,
            UiCommand::LabDraftGet,
            UiCommand::BacktestGet,
            UiCommand::StrategiesList,
            UiCommand::StrategyGet(StrategyParams {
                strategy: ResourceId::parse("low-vol-v1").unwrap()
            }),
            UiCommand::JobsGet,
            UiCommand::BackupList,
            UiCommand::CollectorCredentialOpenNativeSheet(collector),
            UiCommand::CollectorTest(collector),
            UiCommand::CollectorCollect(collector),
            UiCommand::WorkspaceChoose,
            UiCommand::IngestDryRun(path),
            UiCommand::IngestCommit(path),
            UiCommand::BacktestRun,
            UiCommand::BackupCreate(path),
            UiCommand::BackupVerify(path),
            UiCommand::WorkspaceRestoreOpenNativeSheet(path),
            UiCommand::ApprovalRespond(ApprovalRespondParams {
                request: HANDLE.parse().unwrap(),
                decision: ApprovalDecision::Approve
            }),
            UiCommand::TerminalStart,
            UiCommand::TerminalInput(TerminalInputParams {
                data_base64: "bHM=".to_owned()
            }),
            UiCommand::TerminalResize(TerminalResizeParams {
                cols: 120,
                rows: 40
            }),
            UiCommand::TerminalRestart,
        ]
    }

    #[test]
    fn every_command_round_trips()
    {
        for command in every_command()
        {
            let json = serde_json::to_string(&command).unwrap();
            assert_eq!(
                serde_json::from_str::<UiCommand>(&json).unwrap(),
                command,
                "{json}"
            );
        }
    }

    #[test]
    fn the_method_name_is_the_tag_on_the_wire()
    {
        for command in every_command()
        {
            let value = serde_json::to_value(&command).unwrap();
            assert_eq!(value["method"], command.method());
        }
    }

    #[test]
    fn the_command_list_is_the_one_the_design_fixes()
    {
        let methods: Vec<&str> = every_command().iter().map(|c| c.method()).collect();
        let reads: Vec<&str> = every_command()
            .iter()
            .filter(|c| c.kind() == CommandKind::Read)
            .map(|c| c.method())
            .collect();
        let intents: Vec<&str> = every_command()
            .iter()
            .filter(|c| c.kind() == CommandKind::Intent)
            .map(|c| c.method())
            .collect();

        assert_eq!(
            reads,
            [
                "bootstrap.get",
                "today.get",
                "collectors.list",
                "lab.draft.get",
                "backtest.get",
                "strategies.list",
                "strategy.get",
                "jobs.get",
                "backup.list"
            ]
        );
        assert_eq!(
            intents,
            [
                "collector.credential.open_native_sheet",
                "collector.test",
                "collector.collect",
                "workspace.choose",
                "ingest.dry_run",
                "ingest.commit",
                "backtest.run",
                "backup.create",
                "backup.verify",
                "workspace.restore.open_native_sheet",
                "approval.respond",
                "terminal.start",
                "terminal.input",
                "terminal.resize",
                "terminal.restart"
            ]
        );
        assert_eq!(methods.len(), reads.len() + intents.len());
    }

    #[test]
    fn a_command_with_parameters_carries_them_where_the_shape_says()
    {
        let command = UiCommand::StrategyGet(StrategyParams {
            strategy: ResourceId::parse("low-vol-v1").unwrap()
        });
        assert_eq!(
            serde_json::to_string(&command).unwrap(),
            "{\"method\":\"strategy.get\",\"params\":{\"strategy\":\"low-vol-v1\"}}"
        );
    }

    #[test]
    fn an_unknown_method_is_refused()
    {
        assert!(serde_json::from_str::<UiCommand>("{\"method\":\"db.query\"}").is_err());
        assert!(serde_json::from_str::<UiCommand>(
            "{\"method\":\"strategy.get\",\"params\":{\"strategy\":\"../secrets\"}}"
        )
        .is_err());
    }

    #[test]
    fn an_unknown_parameter_is_refused()
    {
        assert!(serde_json::from_str::<UiCommand>(
            "{\"method\":\"terminal.resize\",\"params\":{\"cols\":80,\"rows\":24,\"pixels\":1}}"
        )
        .is_err());
    }

    #[test]
    fn an_envelope_round_trips()
    {
        let envelope = UiCommandEnvelope::new(REQUEST.parse().unwrap(), UiCommand::TodayGet);
        let json = serde_json::to_string(&envelope).unwrap();
        assert_eq!(
            json,
            format!("{{\"v\":1,\"id\":\"{REQUEST}\",\"command\":{{\"method\":\"today.get\"}}}}")
        );
        assert_eq!(
            serde_json::from_str::<UiCommandEnvelope>(&json).unwrap(),
            envelope
        );
    }

    #[test]
    fn a_response_round_trips_either_way()
    {
        let ok: UiResponseEnvelope<u32> = UiResponseEnvelope::ok(REQUEST.parse().unwrap(), 7);
        let json = serde_json::to_string(&ok).unwrap();
        assert_eq!(
            serde_json::from_str::<UiResponseEnvelope<u32>>(&json).unwrap(),
            ok
        );
        assert!(json.contains("\"status\":\"ok\""));

        let failed: UiResponseEnvelope<u32> = UiResponseEnvelope::error(
            REQUEST.parse().unwrap(),
            ErrorEnvelope::new(crate::error::ErrorCode::DbBusy)
        );
        let json = serde_json::to_string(&failed).unwrap();
        assert_eq!(
            serde_json::from_str::<UiResponseEnvelope<u32>>(&json).unwrap(),
            failed
        );
        assert!(json.contains("\"status\":\"error\""));
    }
}
