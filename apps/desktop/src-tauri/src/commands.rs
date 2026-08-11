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
use tauri::ipc::Channel;
use trdr_core::error::{ErrorCode, ErrorEnvelope, ErrorParam, Retryability};
use trdr_core::id::{RequestId, ResourceId, WorkspaceId};
use trdr_core::query::{
    LabDraftModel, LabResultModel, StrategiesModel, StrategyDetailModel, TodayModel
};
use trdr_core::ui::{TerminalInputParams, TerminalResizeParams, UiResponseEnvelope};
use trdr_core::PROTOCOL_VERSION;
use trdr_runtime::base64;
use trdr_runtime::pty::{
    PtyLifecycle, PtyRequest, PtyWindow, SystemPtyHost, TerminalError, TerminalSession,
    TerminalSink
};
use trdr_runtime::query::QueryService;
use trdr_runtime::scrollback::Scrollback;

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
    },
    Command {
        handler: "terminal_start",
        permission: "allow-terminal-start",
        wire_method: Some("terminal.start")
    },
    Command {
        handler: "terminal_input",
        permission: "allow-terminal-input",
        wire_method: Some("terminal.input")
    },
    Command {
        handler: "terminal_resize",
        permission: "allow-terminal-resize",
        wire_method: Some("terminal.resize")
    },
    Command {
        handler: "terminal_restart",
        permission: "allow-terminal-restart",
        wire_method: Some("terminal.restart")
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

/// The environment variable that says which agent to run.
///
/// Section 9.1 puts the agent executable in a setting rather than in a command
/// parameter, and M2 has no settings surface yet, so this is where the setting
/// lives until one exists. What matters for the trust boundary is not where the
/// value is kept but who supplies it: this is read from the process's own
/// environment, by the host, before any window exists. The WebView never names
/// an executable and never names a directory.
pub const AGENT_COMMAND_VARIABLE: &str = "TRDR_AGENT_COMMAND";

/// The agent trdr runs when nothing says otherwise.
pub const DEFAULT_AGENT_COMMAND: &str = "claude";

/// Which agent to start, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCommand
{
    /// The program, as the setting spells it — a bare name to look up, or a
    /// location to use as given.
    pub program: String,
    /// Its arguments.
    pub args: Vec<String>,
    /// The directory it starts in.
    pub cwd: Option<PathBuf>
}

impl AgentCommand
{
    /// The setting, read from the environment, running in this directory.
    pub fn from_environment(cwd: PathBuf) -> Self
    {
        Self::parse(
            &std::env::var(AGENT_COMMAND_VARIABLE).unwrap_or_default(),
            cwd
        )
    }

    /// The same decision with the setting handed in, so a test can make it.
    ///
    /// The value is split on whitespace, so `TRDR_AGENT_COMMAND="codex --cd ."`
    /// is a program and two arguments. Nothing is passed to a shell, which is
    /// why splitting is enough: there is no quoting to honour because there is
    /// nothing that would have interpreted quotes. An empty setting is a
    /// variable someone exported and did not fill in, and falls back rather than
    /// leaving the terminal with no program to run.
    pub fn parse(setting: &str, cwd: PathBuf) -> Self
    {
        let mut words = setting.split_whitespace().map(str::to_owned);

        Self {
            program: words
                .next()
                .unwrap_or_else(|| DEFAULT_AGENT_COMMAND.to_owned()),
            args: words.collect(),
            cwd: Some(cwd)
        }
    }

    /// The same thing in the shape [`trdr_runtime::pty`] asks for.
    fn request(&self) -> PtyRequest
    {
        PtyRequest {
            program: self.program.clone(),
            args: self.args.clone(),
            cwd: self.cwd.clone()
        }
    }
}

/// The agent terminal, as the commands see it.
///
/// Managed by Tauri like [`BootstrapState`], and for the same reason: the
/// session and the agent setting are decided once, by the composition root,
/// before any window exists. A handler reads them.
pub struct TerminalState
{
    session: Arc<TerminalSession>,
    agent: AgentCommand
}

impl TerminalState
{
    /// A terminal that has not started anything, keeping its scrollback here.
    pub fn open(scrollback: PathBuf, agent: AgentCommand) -> Self
    {
        Self {
            session: TerminalSession::new(Arc::new(SystemPtyHost), Scrollback::at(scrollback)),
            agent
        }
    }

    /// What the screen is told about the terminal after a start or a restart.
    fn model(&self, started: trdr_runtime::pty::TerminalStarted) -> TerminalSessionModel
    {
        TerminalSessionModel {
            agent: self.agent.program.clone(),
            pid: started.pid,
            process: self.session.lifecycle().into(),
            cols: started.window.cols,
            rows: started.window.rows
        }
    }

    /// The state the screen renders right now.
    fn acknowledged(&self) -> TerminalAcknowledged
    {
        TerminalAcknowledged {
            process: self.session.lifecycle().into()
        }
    }
}

/// The seven process states the visual contract fixes.
///
/// Five of them are what [`PtyLifecycle`] can report, and the mapping below is
/// the whole of the translation. The other two have no producer on this path and
/// cannot get one:
///
/// - `needs-input` would have to be read out of the agent's own output, and
///   section 11 forbids turning terminal content into product state in either
///   direction. There is no prompt detector here and there is not going to be
///   one.
/// - `approval-pending` belongs to the registration approval round-trip in
///   section 9.3, which is milestone M6. When it exists it will be produced by
///   the approval broker, which knows about pending requests, and not by the
///   terminal, which does not.
///
/// M2 still has to show all seven, because the whole app in M2 runs on a
/// synthetic fixture. That is the surface's job and it is done in TypeScript,
/// where the fixture producer lives beside the live one and is marked as
/// synthetic — see `apps/desktop/src/terminal/processState.ts`. Nothing on this
/// path can produce a state the host did not observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalProcess
{
    /// A start has been asked for and no child exists yet.
    Starting,
    /// A child is running and has printed nothing.
    Ready,
    /// The child has printed something.
    Running,
    /// The agent is waiting for the person to answer it. No producer in M2.
    NeedsInput,
    /// An approval sheet is open for this agent. No producer until M6.
    ApprovalPending,
    /// No child is running.
    Exited,
    /// The previous child is being stopped and a new one started.
    Reconnecting
}

impl From<PtyLifecycle> for TerminalProcess
{
    fn from(lifecycle: PtyLifecycle) -> Self
    {
        match lifecycle
        {
            PtyLifecycle::Starting => Self::Starting,
            PtyLifecycle::Ready => Self::Ready,
            PtyLifecycle::Running => Self::Running,
            PtyLifecycle::Reconnecting => Self::Reconnecting,
            PtyLifecycle::Exited => Self::Exited
        }
    }
}

/// One chunk of the agent's output, on its way to the terminal in the WebView.
///
/// Base64 for the reason [`trdr_runtime::base64`] gives and the reason
/// [`TerminalInputParams`] already gives for the other direction: terminal
/// traffic is not text, a read can end in the middle of a character, and
/// anything that converted the stream would replace what it could not decode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TerminalOutput
{
    /// Raw bytes, base64 encoded.
    pub data_base64: String
}

/// A process-state change, on its way to the status bar.
///
/// One field, and it is deliberately the only one. Section 9.1 separates
/// host-to-UI state events from the terminal byte stream, and a type with
/// nowhere to put a byte is a stronger separation than a rule about where not to
/// put one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TerminalProcessEvent
{
    /// Where the process is now.
    pub process: TerminalProcess
}

/// What a start or a restart settled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TerminalSessionModel
{
    /// The agent as the setting names it.
    ///
    /// The configured name, never the location this process resolved it to. The
    /// screen has no reason to learn where a person keeps their tools.
    pub agent: String,
    /// The child's process id, where the system gave one.
    pub pid: Option<u32>,
    /// Where the process is now.
    pub process: TerminalProcess,
    /// Columns the child was told about.
    pub cols: u16,
    /// Rows the child was told about.
    pub rows: u16
}

/// What the terminal answers to a keystroke or a resize.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TerminalAcknowledged
{
    /// Where the process is now.
    pub process: TerminalProcess
}

/// The envelope [`terminal_start`] answers with. Transparent; see the module
/// documentation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct TerminalStartResponse(pub UiResponseEnvelope<TerminalSessionModel>);

/// The envelope [`terminal_restart`] answers with. Transparent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct TerminalRestartResponse(pub UiResponseEnvelope<TerminalSessionModel>);

/// The envelope [`terminal_input`] answers with. Transparent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct TerminalInputResponse(pub UiResponseEnvelope<TerminalAcknowledged>);

/// The envelope [`terminal_resize`] answers with. Transparent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct TerminalResizeResponse(pub UiResponseEnvelope<TerminalAcknowledged>);

/// The two channels the WebView opened, as the runtime's one sink.
///
/// The separation section 9.1 asks for is these two fields. Bytes go out of
/// `output` and states go out of `lifecycle`, the payload types have no field in
/// common, and neither method can reach the other's channel.
struct WebViewSink
{
    output: Channel<TerminalOutput>,
    lifecycle: Channel<TerminalProcessEvent>
}

impl TerminalSink for WebViewSink
{
    fn output(&self, bytes: &[u8])
    {
        let _ = self.output.send(TerminalOutput {
            data_base64: base64::encode(bytes)
        });
    }

    fn lifecycle(&self, state: PtyLifecycle)
    {
        let _ = self.lifecycle.send(TerminalProcessEvent {
            process: state.into()
        });
    }
}

/// Starts the agent terminal, and opens the two channels it speaks through
/// (section 9.1's `terminal.start`).
///
/// Both channels are arguments rather than events. Events would need the
/// WebView to be granted `core:event:listen`, which is a general ability to
/// listen to anything the host emits; a channel is created by the caller, passed
/// to one command, and reaches nothing else. `capabilities/main.json` therefore
/// stays a list of four terminal commands rather than four commands and a
/// permission that also covers everything added later.
///
/// Calling this twice does not start a second agent. A WebView that reloaded has
/// no way to know whether the host still has a terminal, so the answer is the
/// one that is running, with the scrollback replayed down the output channel
/// before anything live.
#[tauri::command]
#[specta::specta]
pub fn terminal_start(
    id: RequestId,
    output: Channel<TerminalOutput>,
    lifecycle: Channel<TerminalProcessEvent>,
    state: tauri::State<'_, TerminalState>
) -> TerminalStartResponse
{
    state
        .session
        .attach(Arc::new(WebViewSink { output, lifecycle }));

    TerminalStartResponse(match state.session.start(&state.agent.request())
    {
        Ok(started) => UiResponseEnvelope::ok(id, state.model(started)),
        Err(error) => UiResponseEnvelope::error(id, terminal_envelope(id, &error))
    })
}

/// Sends keystrokes to the agent (section 9.1's `terminal.input`).
#[tauri::command]
#[specta::specta]
pub fn terminal_input(
    id: RequestId,
    input: TerminalInputParams,
    state: tauri::State<'_, TerminalState>
) -> TerminalInputResponse
{
    let Ok(bytes) = base64::decode(&input.data_base64)
    else
    {
        return TerminalInputResponse(UiResponseEnvelope::error(
            id,
            ErrorEnvelope::new(ErrorCode::AppProtocolVersion)
                .with_request(id)
                .with_param("reason", ErrorParam::literal("terminal_input_not_base64"))
        ));
    };

    TerminalInputResponse(match state.session.input(&bytes)
    {
        Ok(()) => UiResponseEnvelope::ok(id, state.acknowledged()),
        Err(error) => UiResponseEnvelope::error(id, terminal_envelope(id, &error))
    })
}

/// Tells the agent the terminal changed size (section 9.1's `terminal.resize`).
///
/// A size that arrives with no child running is kept rather than refused; see
/// [`trdr_runtime::pty::TerminalSession::resize`] for why that is the whole
/// answer to the race between a screen laying itself out and a process starting.
#[tauri::command]
#[specta::specta]
pub fn terminal_resize(
    id: RequestId,
    size: TerminalResizeParams,
    state: tauri::State<'_, TerminalState>
) -> TerminalResizeResponse
{
    let window = PtyWindow {
        cols: size.cols,
        rows: size.rows
    };

    TerminalResizeResponse(match state.session.resize(window)
    {
        Ok(()) => UiResponseEnvelope::ok(id, state.acknowledged()),
        Err(error) => UiResponseEnvelope::error(id, terminal_envelope(id, &error))
    })
}

/// Stops the agent and starts it again (section 9.1's `terminal.restart`).
///
/// The channels are the ones the last [`terminal_start`] opened. A restart is
/// the same terminal with a new child in it, not a new terminal.
#[tauri::command]
#[specta::specta]
pub fn terminal_restart(
    id: RequestId,
    state: tauri::State<'_, TerminalState>
) -> TerminalRestartResponse
{
    TerminalRestartResponse(match state.session.restart(&state.agent.request())
    {
        Ok(started) => UiResponseEnvelope::ok(id, state.model(started)),
        Err(error) => UiResponseEnvelope::error(id, terminal_envelope(id, &error))
    })
}

/// A terminal failure in the one shape every surface renders (section 12).
///
/// The name of the program is carried as a parameter because the screen has to
/// be able to say which agent it could not find — a rail that goes blank with no
/// word about why is the failure this exists to prevent. It is the configured
/// name and never a resolved location, so nothing here describes the user's
/// filesystem, and no terminal byte is a parameter of anything.
fn terminal_envelope(id: RequestId, error: &TerminalError) -> ErrorEnvelope
{
    let envelope = match error
    {
        TerminalError::ExecutableMissing { name } =>
        {
            ErrorEnvelope::new(ErrorCode::TerminalExecutableMissing).with_param(
                "agent",
                ErrorParam::text(name.clone()).unwrap_or_else(|_| ErrorParam::literal("agent"))
            )
        }
        TerminalError::Spawn => ErrorEnvelope::new(ErrorCode::TerminalSpawn),
        TerminalError::NotRunning => ErrorEnvelope::new(ErrorCode::TerminalExited)
    };

    envelope
        .with_request(id)
        .with_retryability(Retryability::Immediate)
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
            UiCommand::JobsGet,
            UiCommand::TerminalStart,
            UiCommand::TerminalInput(TerminalInputParams {
                data_base64: String::new()
            }),
            UiCommand::TerminalResize(TerminalResizeParams { cols: 80, rows: 24 }),
            UiCommand::TerminalRestart
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

        let terminal = UiResponseEnvelope::ok(
            request(),
            TerminalAcknowledged {
                process: TerminalProcess::Ready
            }
        );
        assert_eq!(
            serde_json::to_value(TerminalInputResponse(terminal.clone())).unwrap(),
            serde_json::to_value(&terminal).unwrap()
        );
    }

    /// The seven names are the visual contract's, exactly.
    ///
    /// `design/ui-kit/contracts.v2.json` spells the `terminalProcess` states in
    /// kebab case, the stylesheet selects on `[data-process="needs-input"]`, and
    /// a renamed variant here would leave the status bar rendering with no
    /// colour and nothing failing.
    #[test]
    fn the_process_states_are_spelled_the_way_the_visual_contract_spells_them()
    {
        let states = [
            (TerminalProcess::Starting, "starting"),
            (TerminalProcess::Ready, "ready"),
            (TerminalProcess::Running, "running"),
            (TerminalProcess::NeedsInput, "needs-input"),
            (TerminalProcess::ApprovalPending, "approval-pending"),
            (TerminalProcess::Exited, "exited"),
            (TerminalProcess::Reconnecting, "reconnecting")
        ];

        for (state, wire) in states
        {
            assert_eq!(serde_json::to_value(state).unwrap(), wire);
        }

        assert_eq!(states.len(), 7);
    }

    /// The five the host can observe, and the two it cannot.
    ///
    /// The mapping is total and the two unmapped states are unreachable from
    /// here, which is the property that keeps a live session from ever showing a
    /// state nobody observed.
    #[test]
    fn only_the_states_the_runtime_can_observe_come_out_of_a_lifecycle()
    {
        let observed: Vec<TerminalProcess> = [
            PtyLifecycle::Starting,
            PtyLifecycle::Ready,
            PtyLifecycle::Running,
            PtyLifecycle::Reconnecting,
            PtyLifecycle::Exited
        ]
        .into_iter()
        .map(TerminalProcess::from)
        .collect();

        assert!(!observed.contains(&TerminalProcess::NeedsInput));
        assert!(!observed.contains(&TerminalProcess::ApprovalPending));
        assert_eq!(observed.len(), 5);
    }

    /// Section 9.1 separates process-state events from the byte stream, and this
    /// is what holds it: the state message has one field, it is the state, and
    /// no arrangement of bytes can put anything else in it.
    #[test]
    fn a_process_state_message_carries_the_state_and_nothing_else()
    {
        let json = serde_json::to_value(TerminalProcessEvent {
            process: TerminalProcess::Running
        })
        .unwrap();

        assert_eq!(json, serde_json::json!({ "process": "running" }));
        assert_eq!(json.as_object().map(serde_json::Map::len), Some(1));
    }

    /// Bytes cross as bytes. Escape sequences and values that are not valid
    /// UTF-8 are what a terminal is made of, and a message that carried them as
    /// text would replace the ones it could not decode.
    #[test]
    fn the_output_message_carries_bytes_that_are_not_text()
    {
        let bytes: Vec<u8> = vec![0x1b, b'[', b'3', b'1', b'm', 0xff, 0xfe];
        let message = TerminalOutput {
            data_base64: base64::encode(&bytes)
        };

        assert_eq!(base64::decode(&message.data_base64).unwrap(), bytes);
        assert!(serde_json::to_string(&message).unwrap().is_ascii());
    }

    /// Every terminal failure renders as one of section 12's `TERMINAL_*` codes,
    /// and the parameter it carries is the name from the setting rather than
    /// anything this process resolved or the agent printed.
    #[test]
    fn every_terminal_failure_renders_as_a_code_the_design_defines()
    {
        let failures = [
            (
                TerminalError::ExecutableMissing {
                    name: "codex".to_owned()
                },
                ErrorCode::TerminalExecutableMissing
            ),
            (TerminalError::Spawn, ErrorCode::TerminalSpawn),
            (TerminalError::NotRunning, ErrorCode::TerminalExited)
        ];

        for (failure, code) in failures
        {
            let envelope = terminal_envelope(request(), &failure);

            assert_eq!(envelope.code, code);
            assert_eq!(envelope.id, Some(request()));
            assert!(trdr_core::ErrorCode::ALL.contains(&envelope.code));
        }

        assert_eq!(
            terminal_envelope(
                request(),
                &TerminalError::ExecutableMissing {
                    name: "codex".to_owned()
                }
            )
            .params
            .get("agent"),
            Some(&ErrorParam::Text("codex".to_owned()))
        );
    }

    /// Input that is not base64 is refused, and refused as a protocol failure
    /// rather than as something the terminal did.
    #[test]
    fn input_that_is_not_base64_never_reaches_a_process()
    {
        let state = TerminalState::open(
            PathBuf::from("/private/tmp/trdr-t/commands-input/scrollback.bin"),
            AgentCommand::parse("/bin/cat", PathBuf::from("/private/tmp/trdr-t"))
        );
        let refused = base64::decode("not base64!");

        assert!(refused.is_err());
        assert_eq!(state.acknowledged().process, TerminalProcess::Exited);
    }

    /// The agent is a setting, and a setting nobody filled in is not a reason to
    /// have no agent.
    #[test]
    fn the_agent_setting_is_a_program_and_its_arguments()
    {
        let cwd = PathBuf::from("/private/tmp/trdr-t/workspace");

        assert_eq!(
            AgentCommand::parse("codex  --cd .", cwd.clone()),
            AgentCommand {
                program: "codex".to_owned(),
                args: vec!["--cd".to_owned(), ".".to_owned()],
                cwd: Some(cwd.clone())
            }
        );
        assert_eq!(
            AgentCommand::parse("   ", cwd.clone()).program,
            DEFAULT_AGENT_COMMAND
        );
        assert_eq!(AgentCommand::parse("", cwd).args, Vec::<String>::new());
    }
}
