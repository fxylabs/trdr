//! Every command the main WebView can call.
//!
//! `docs/FOUNDATION_DESIGN.md` section 9.1 fixes the whole list of commands the
//! screen will eventually reach, and `trdr_core::ui::UiCommand` writes that list
//! down. This module implements two of them. The rest are not stubbed out here:
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
//! # Nothing here touches the outside world
//!
//! These handlers read no database, discover no workspace, open no socket, and
//! do not create `~/.trdr`. Everything they return is a constant of this build.
//! The tracks that own storage and process supervision fill them in.
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
use trdr_core::id::RequestId;
use trdr_core::ui::UiResponseEnvelope;
use trdr_core::PROTOCOL_VERSION;

/// The version of the app itself, as the crate manifest states it.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
    }
];

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
/// The shape is small because everything else it will eventually carry — which
/// workspace is open, whether its database migrated, which collectors have
/// credentials — has to be read from disk, and this build reads nothing. The
/// fields here are the ones that are true of the binary itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct BootstrapModel
{
    /// The IPC protocol version this build speaks.
    pub protocol_version: u16,
    /// The app's own version.
    pub app_version: String,
    /// Whether a workspace is open.
    ///
    /// Always `false` here, and truthfully so: workspace discovery belongs to
    /// another track, and this build deliberately neither creates nor reads
    /// `~/.trdr`. The screen already has to handle `false` — it is the state a
    /// first run is in — so filling this in later changes no screen logic.
    pub workspace_open: bool
}

/// The envelope [`bootstrap_get`] answers with.
///
/// Transparent, for the reason the module documentation gives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(transparent)]
pub struct BootstrapResponse(pub UiResponseEnvelope<BootstrapModel>);

/// Answers `bootstrap.get` with what this build can know without touching disk.
#[tauri::command]
#[specta::specta]
pub fn bootstrap_get(id: RequestId) -> BootstrapResponse
{
    BootstrapResponse(UiResponseEnvelope::ok(
        id,
        BootstrapModel {
            protocol_version: PROTOCOL_VERSION,
            app_version: APP_VERSION.to_owned(),
            workspace_open: false
        }
    ))
}

#[cfg(test)]
mod tests
{
    use super::*;
    use trdr_core::ui::{UiCommand, UiOutcome};

    const REQUEST: &str = "01KZNNR5X818P3J6ENYKSADP8W";

    fn request() -> RequestId
    {
        REQUEST.parse().unwrap()
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
    fn bootstrap_reports_the_build_and_opens_no_workspace()
    {
        let BootstrapResponse(answer) = bootstrap_get(request());

        let UiOutcome::Ok(model) = answer.outcome
        else
        {
            panic!("the stub cannot fail");
        };

        assert_eq!(model.protocol_version, PROTOCOL_VERSION);
        assert_eq!(model.app_version, APP_VERSION);
        assert!(
            !model.workspace_open,
            "this track must not discover or create a workspace"
        );
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
            UiCommand::StrategiesList,
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
        let json = serde_json::to_value(bootstrap_get(request())).unwrap();

        assert_eq!(json["v"], 1);
        assert_eq!(json["id"], REQUEST);
        assert_eq!(json["outcome"]["status"], "ok");
        assert_eq!(json["outcome"]["value"]["workspace_open"], false);
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

        let BootstrapResponse(inner) = bootstrap_get(request());
        assert_eq!(
            serde_json::to_value(bootstrap_get(request())).unwrap(),
            serde_json::to_value(&inner).unwrap()
        );
    }
}
