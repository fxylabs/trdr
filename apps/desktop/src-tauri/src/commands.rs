//! Every command the main WebView can call.
//!
//! `docs/FOUNDATION_DESIGN.md` section 9.1 fixes the whole list of commands the
//! screen will eventually reach, and `trdr_core::ui::UiCommand` writes that list
//! down. This module implements the read commands milestone M2 needs. The rest
//! are not stubbed out here:
//! an unimplemented command that is registered is still a reachable command, and
//! the capability file is only as narrow as the surface behind it.
//!
//! Three rules hold for everything in this module, and the tests alongside are
//! what keep them holding.
//!
//! # A command answers with an envelope, never with a `Result`
//!
//! Tauri turns `Err` into a rejected promise, which would give trdr a second
//! error channel beside [`trdr_core::ErrorEnvelope`] — and a screen that has to
//! read failures out of two places will eventually read one of them wrong. So
//! every command returns [`UiResponseEnvelope`] infallibly, and the failure case
//! is the `error` arm of [`trdr_core::ui::UiOutcome`] carrying a real error
//! envelope. The generated TypeScript follows — the promise resolves to the
//! envelope and never rejects.
//!
//! # The Rust name and the wire name are checked against each other
//!
//! A Tauri command name is a Rust identifier, so `bootstrap.get` from section
//! 9.1 is spelled `bootstrap_get` here. That is a translation, and a translation
//! that nothing checks is a translation that drifts, so
//! [`Command::wire_method`] carries the section 9.1 spelling and a test asserts
//! it against `UiCommand::method()`.
//!
//! # A handler reads, it does not discover
//!
//! Where the workspace is, which one it is, and what schema its database is at
//! are decided once, by [`crate::startup`], before any window exists. A handler
//! reads that decision out of [`BootstrapState`] rather than going to look for
//! itself. Two things follow: no command can open a second database or take a
//! second lease by accident, and a test can hand a handler a state it made up
//! instead of a real product root — which is why nothing in this file's tests
//! goes near `~/.trdr`.
//!
//! # Why each command has a named response type
//!
//! A handler here returns `PingResponse`, not `UiResponseEnvelope<Pong>`, and
//! that is working around a bug rather than expressing a design. tauri-specta
//! 2.0.0-rc.25 drops the concrete type arguments when it splits a generic return
//! type into its serialize and deserialize forms, and emits
//! `UiResponseEnvelope_Serialize<T>` with `T` unbound — TypeScript that does not
//! compile. The same instantiation in field position is substituted correctly,
//! so a `#[serde(transparent)]` newtype puts the generic one level down and the
//! generated binding comes out as `UiResponseEnvelope_Serialize<Pong>`.
//!
//! `transparent` means the bytes on the wire are the envelope's own and nothing
//! is added, and [`tests::a_named_response_is_the_envelope_and_nothing_more`]
//! holds that. When tauri-specta substitutes return-position generics, these two
//! types can be deleted and the handlers can name the envelope directly, with no
//! change to what the WebView receives.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use trdr_core::error::ErrorEnvelope;
use trdr_core::id::{RequestId, ResourceId, WorkspaceId};
use trdr_core::query::{
    LabDraftModel, LabResultModel, StrategiesModel, StrategyDetailModel, TodayModel
};
use trdr_core::ui::UiResponseEnvelope;
use trdr_core::PROTOCOL_VERSION;
use trdr_runtime::query::QueryService;

/// The version of the app itself, as the crate manifest states it.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// What start-up settled, for the commands that answer from it.
///
/// Managed by Tauri, so a handler receives it as `State` and cannot construct
/// one — which is the point. The values are read once, from the workspace this
/// process opened under the writer lease, and nothing that runs later can
/// disagree with them without the app having restarted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapState
{
    /// The portable identity in this workspace's `workspace.json`.
    pub workspace_id: WorkspaceId,
    /// Where the workspace directory is.
    pub workspace_path: PathBuf,
    /// The schema version the open database reported.
    pub schema_version: i32
}

/// One registered command, named on both sides of the boundary.
///
/// The Tauri handler, the generated bindings, the capability file, and the build
/// script each need this command's name in a slightly different form. Keeping
/// the forms together in one value is what lets a test compare them instead of a
/// reader remembering to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Command
{
    /// The Rust function, which is also the name Tauri dispatches on.
    pub handler: &'static str,
    /// The permission that must appear in `capabilities/main.json` for the main
    /// WebView to be allowed to call it.
    pub permission: &'static str,
    /// The name section 9.1 gives this command, or `None` where the command is
    /// this crate's own rather than part of that contract.
    pub wire_method: Option<&'static str>
}

/// Every command registered with Tauri, and nothing else.
///
/// The order matches `collect_commands!` in [`crate::builder`].
pub const COMMANDS: &[Command] = &[
    Command {
        handler: "ping",
        permission: "allow-ping",
        wire_method: None
    },
    Command {
        handler: "bootstrap_get",
        permission: "allow-bootstrap-get",
        wire_method: Some("bootstrap.get")
    },
    Command {
        handler: "today_get",
        permission: "allow-today-get",
        wire_method: Some("today.get")
    },
    Command {
        handler: "lab_draft_get",
        permission: "allow-lab-draft-get",
        wire_method: Some("lab.draft.get")
    },
    Command {
        handler: "backtest_get",
        permission: "allow-backtest-get",
        wire_method: Some("backtest.get")
    },
    Command {
        handler: "strategies_list",
        permission: "allow-strategies-list",
        wire_method: Some("strategies.list")
    },
    Command {
        handler: "strategy_get",
        permission: "allow-strategy-get",
        wire_method: Some("strategy.get")
    }
];

/// The query service every screen's model comes out of.
///
/// A trait object rather than the concrete [`trdr_runtime::query::SyntheticQueries`],
/// so that this crate holds no opinion about where a model's data came from.
/// Milestone M2 manages the synthetic one; the database-backed one M3 brings is
/// a different value behind the same type, and none of the handlers below change.
pub struct Queries(pub Arc<dyn QueryService>);

/// Turns what the query service returned into the envelope the WebView reads.
///
/// The failure arm is why this exists rather than each handler mapping for
/// itself: an [`ErrorEnvelope`] built by the runtime does not know which request
/// it is answering, and a screen that receives an error with no request id
/// cannot match it to the call it made. Stamping the id in one place is what
/// keeps every handler from having to remember to.
fn answer<T>(id: RequestId, result: Result<T, ErrorEnvelope>) -> UiResponseEnvelope<T>
{
    match result
    {
        Ok(value) => UiResponseEnvelope::ok(id, value),
        Err(error) => UiResponseEnvelope::error(id, error.with_request(id))
    }
}

/// What [`ping`] answers with.
///
/// Deliberately not part of `trdr-core`: this is a liveness check on the bridge
/// between the WebView and the host, not a contract between the app and the CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct Pong
{
    /// The protocol version this build speaks, so the screen learns it from the
    /// same round-trip that proved the bridge works.
    pub protocol_version: u16
}

/// The envelope [`ping`] answers with, named so the bindings can describe it.
///
/// Transparent: this is `UiResponseEnvelope<Pong>` on the wire and in the
/// generated TypeScript alike. See the module documentation for why the name
/// exists at all.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct PingResponse(pub UiResponseEnvelope<Pong>);

/// A typed round-trip, and the proof that the bridge carries validated types.
///
/// The argument is a [`RequestId`], not a string. That is what makes this a real
/// test of the boundary rather than a test of `invoke`: `RequestId` crosses to
/// TypeScript as a plain `string`, and is parsed back into a 26-character ULID
/// on the way in, so a WebView that sends anything else is refused by
/// deserialisation before this function is entered.
#[tauri::command]
#[specta::specta]
pub fn ping(id: RequestId) -> PingResponse
{
    PingResponse(UiResponseEnvelope::ok(
        id,
        Pong {
            protocol_version: PROTOCOL_VERSION
        }
    ))
}

/// What the app needs before it can show anything (section 9.1's `bootstrap.get`).
///
/// Five fields, and the list is meant to stay short. Section 11 gives every
/// screen its own query model; this is only what has to be true before the first
/// screen can be drawn at all — which workspace is open, what build is running,
/// and what the two versions on the wire are.
///
/// # The workspace path, and why a screen is allowed to see one
///
/// Section 9.1 forbids a screen from *passing* a path: it names a location with
/// a [`trdr_core::id::ScopedPathHandle`] the host minted after a native picker,
/// so it can never name a file the user did not choose. Being told where the
/// open workspace is, is the other direction and a different question. The host
/// chose it, the person is entitled to know it, and the alternative — a screen
/// that cannot say which workspace it is showing — is worse. What stays out of
/// reach is the general ability to resolve paths, which is why the capability
/// file grants no `core:path` permission and `tests/capability.rs` checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BootstrapModel
{
    /// The IPC protocol version this build speaks.
    pub protocol_version: u16,
    /// The app's own version.
    pub app_version: String,
    /// The portable identity of the workspace that is open.
    pub workspace_id: WorkspaceId,
    /// Where that workspace is on disk.
    pub workspace_path: String,
    /// The schema version of its database.
    pub schema_version: i32
}

/// The envelope [`bootstrap_get`] answers with.
///
/// Transparent, for the reason the module documentation gives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct BootstrapResponse(pub UiResponseEnvelope<BootstrapModel>);

/// Answers `bootstrap.get` from the workspace this process opened at start-up.
#[tauri::command]
#[specta::specta]
pub fn bootstrap_get(id: RequestId, state: tauri::State<'_, BootstrapState>) -> BootstrapResponse
{
    BootstrapResponse(UiResponseEnvelope::ok(id, bootstrap_model(&state)))
}

/// The model, built from the state, with no Tauri around it.
///
/// Split out so the shape can be tested without a running app. The path is
/// rendered with `display` rather than serialised as a `PathBuf`: a path that is
/// not valid UTF-8 makes serde fail, and a bootstrap that cannot answer is worse
/// than one that answers with a path spelled with a replacement character.
fn bootstrap_model(state: &BootstrapState) -> BootstrapModel
{
    BootstrapModel {
        protocol_version: PROTOCOL_VERSION,
        app_version: APP_VERSION.to_owned(),
        workspace_id: state.workspace_id,
        workspace_path: state.workspace_path.display().to_string(),
        schema_version: state.schema_version
    }
}

/// The envelope [`today_get`] answers with.
///
/// Transparent, for the reason the module documentation gives. The same holds
/// for the four below it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct TodayResponse(pub UiResponseEnvelope<TodayModel>);

/// Answers section 9.1's `today.get` with `TodayModel/v1`.
#[tauri::command]
#[specta::specta]
pub fn today_get(id: RequestId, queries: tauri::State<'_, Queries>) -> TodayResponse
{
    TodayResponse(answer(id, queries.0.today()))
}

/// The envelope [`lab_draft_get`] answers with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct LabDraftResponse(pub UiResponseEnvelope<LabDraftModel>);

/// Answers section 9.1's `lab.draft.get` with `LabDraftModel/v1`.
#[tauri::command]
#[specta::specta]
pub fn lab_draft_get(id: RequestId, queries: tauri::State<'_, Queries>) -> LabDraftResponse
{
    LabDraftResponse(answer(id, queries.0.lab_draft()))
}

/// The envelope [`backtest_get`] answers with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct BacktestResponse(pub UiResponseEnvelope<LabResultModel>);

/// Answers section 9.1's `backtest.get` with `LabResultModel/v1`.
///
/// The command is `backtest.get` and the model is `LabResultModel/v1`, and the
/// two names disagreeing is section 9.1 and section 11 each naming the thing
/// from where they stand: the command asks for a backtest, and the Lab is the
/// screen that shows one. Renaming either to match would put this crate's
/// convenience above two contracts.
#[tauri::command]
#[specta::specta]
pub fn backtest_get(id: RequestId, queries: tauri::State<'_, Queries>) -> BacktestResponse
{
    BacktestResponse(answer(id, queries.0.lab_result()))
}

/// The envelope [`strategies_list`] answers with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct StrategiesResponse(pub UiResponseEnvelope<StrategiesModel>);

/// Answers section 9.1's `strategies.list` with `StrategiesModel/v1`.
#[tauri::command]
#[specta::specta]
pub fn strategies_list(id: RequestId, queries: tauri::State<'_, Queries>) -> StrategiesResponse
{
    StrategiesResponse(answer(id, queries.0.strategies()))
}

/// The envelope [`strategy_get`] answers with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct StrategyResponse(pub UiResponseEnvelope<StrategyDetailModel>);

/// Answers section 9.1's `strategy.get` with `StrategyDetailModel/v1`.
///
/// The argument is a [`ResourceId`], not a string, for the reason [`ping`]'s is
/// a [`RequestId`]: the charset and length rules are enforced by
/// deserialisation, so a WebView that sends a path, an empty string, or a
/// kilobyte of text is refused before this function is entered.
#[tauri::command]
#[specta::specta]
pub fn strategy_get(
    id: RequestId,
    strategy: ResourceId,
    queries: tauri::State<'_, Queries>
) -> StrategyResponse
{
    StrategyResponse(answer(id, queries.0.strategy(&strategy)))
}

#[cfg(test)]
mod tests
{
    use super::*;
    use trdr_core::ui::{UiCommand, UiOutcome};

    const REQUEST: &str = "01KZNNR5X818P3J6ENYKSADP8W";
    const WORKSPACE: &str = "01KZNP0GQ3X8ARK9DQ489Z7WJ8";

    fn request() -> RequestId
    {
        REQUEST.parse().unwrap()
    }

    /// A state no product root was consulted for. Every test in this module
    /// answers from one of these, so none of them can reach `~/.trdr`.
    pub(super) fn state() -> BootstrapState
    {
        BootstrapState {
            workspace_id: WORKSPACE.parse().unwrap(),
            workspace_path: PathBuf::from("/private/tmp/trdr-t/x/workspaces/default"),
            schema_version: 1
        }
    }

    #[test]
    fn ping_answers_the_request_it_was_given()
    {
        let PingResponse(answer) = ping(request());

        assert_eq!(answer.id, request());
        assert_eq!(answer.v.get(), PROTOCOL_VERSION);
        assert_eq!(
            answer.outcome,
            UiOutcome::Ok(Pong {
                protocol_version: PROTOCOL_VERSION
            })
        );
    }

    #[test]
    fn bootstrap_reports_the_workspace_start_up_opened()
    {
        let state = state();
        let model = bootstrap_model(&state);

        assert_eq!(model.protocol_version, PROTOCOL_VERSION);
        assert_eq!(model.app_version, APP_VERSION);
        assert_eq!(model.workspace_id, state.workspace_id);
        assert_eq!(
            model.workspace_path,
            "/private/tmp/trdr-t/x/workspaces/default"
        );
        assert_eq!(model.schema_version, 1);
    }

    /// The id is what the screen shows and what a backup carries, so it has to
    /// survive the crossing as the same 26 characters rather than as whatever a
    /// `Debug` impl would print.
    #[test]
    fn the_workspace_id_crosses_as_the_text_it_is_written_as()
    {
        let json = serde_json::to_value(bootstrap_model(&state())).unwrap();

        assert_eq!(json["workspace_id"], WORKSPACE);
    }

    /// The translation between a Rust identifier and section 9.1's dotted name
    /// is the kind of thing that is right once and wrong after the next rename.
    #[test]
    fn a_wire_method_is_the_one_the_command_contract_names()
    {
        let known: Vec<&str> = [
            UiCommand::BootstrapGet,
            UiCommand::TodayGet,
            UiCommand::CollectorsList,
            UiCommand::LabDraftGet,
            UiCommand::BacktestGet,
            UiCommand::StrategiesList,
            UiCommand::StrategyGet(trdr_core::ui::StrategyParams {
                strategy: ResourceId::parse("syn-meanrev").unwrap()
            }),
            UiCommand::JobsGet
        ]
        .iter()
        .map(|command| command.method())
        .collect();

        for command in COMMANDS
        {
            let Some(method) = command.wire_method
            else
            {
                continue;
            };

            assert!(
                known.contains(&method),
                "{method} is not a command section 9.1 defines"
            );
            assert_eq!(
                method.replace('.', "_"),
                command.handler,
                "the handler name and the wire name disagree"
            );
        }
    }

    /// A permission identifier Tauri does not generate is a permission that
    /// silently does nothing, so the spelling rule is asserted rather than
    /// trusted: `allow-` followed by the handler name with underscores turned
    /// into dashes.
    #[test]
    fn a_permission_is_spelled_the_way_tauri_generates_it()
    {
        for command in COMMANDS
        {
            assert_eq!(
                command.permission,
                format!("allow-{}", command.handler.replace('_', "-"))
            );
        }
    }

    #[test]
    fn a_response_reaches_the_webview_as_the_envelope_the_design_fixes()
    {
        let answer =
            BootstrapResponse(UiResponseEnvelope::ok(request(), bootstrap_model(&state())));
        let json = serde_json::to_value(answer).unwrap();

        assert_eq!(json["v"], 1);
        assert_eq!(json["id"], REQUEST);
        assert_eq!(json["outcome"]["status"], "ok");
        assert_eq!(json["outcome"]["value"]["workspace_id"], WORKSPACE);
    }

    /// The named response types exist to work around a generator bug, so the one
    /// thing that must never become true of them is that they change the wire.
    /// Drop `#[serde(transparent)]` from either and this fails.
    #[test]
    fn a_named_response_is_the_envelope_and_nothing_more()
    {
        let PingResponse(inner) = ping(request());
        assert_eq!(
            serde_json::to_value(ping(request())).unwrap(),
            serde_json::to_value(&inner).unwrap()
        );

        let envelope = UiResponseEnvelope::ok(request(), bootstrap_model(&state()));
        assert_eq!(
            serde_json::to_value(BootstrapResponse(envelope.clone())).unwrap(),
            serde_json::to_value(&envelope).unwrap()
        );
    }
}
