//! What the main WebView can and cannot reach, checked against the real thing.
//!
//! `capabilities/main.json` is a JSON file, and a JSON file that nothing
//! exercises is a claim rather than a control. These tests build the app from
//! the shipped `tauri.conf.json`, open the window the shipped config declares,
//! and push messages through the same IPC path a WebView uses — so what they
//! prove is what the access-control list actually resolves to, not what the file
//! looks like.
//!
//! Two failures they exist to catch:
//!
//! - A capability that quietly grows. `core:default` alone would hand the screen
//!   window manipulation, path resolution, event emission, and tray and menu
//!   control. [`the_webview_cannot_reach_a_core_plugin_command`] fails the
//!   moment any of that is granted.
//! - An application command that is registered but not declared in `build.rs`.
//!   Tauri only applies the access-control list to app commands when an
//!   application manifest exists, so dropping that declaration would silently
//!   make every command callable from every WebView, and
//!   [`an_unregistered_application_command_is_refused`] is what notices.

use tauri::ipc::CallbackFn;
use tauri::test::{get_ipc_response, mock_builder, INVOKE_KEY};
use tauri::utils::config::WindowConfig;
use tauri::webview::InvokeRequest;
use tauri::{App, WebviewWindow, WebviewWindowBuilder};

const REQUEST: &str = "01KZNNR5X818P3J6ENYKSADP8W";

/// The app as it ships, on the runtime the tests can drive.
fn app() -> App<tauri::test::MockRuntime>
{
    mock_builder()
        .invoke_handler(trdr_desktop_lib::builder::commands().invoke_handler())
        .build(tauri::generate_context!())
        .expect("the shipped config should build")
}

/// The window `tauri.conf.json` declares, opened the way the app opens it.
///
/// Built from the config rather than from a literal label, because the label is
/// what ties this window to the capability file — a test that hard-coded `"main"`
/// would keep passing after the config renamed the window out from under it.
fn main_window(app: &App<tauri::test::MockRuntime>) -> WebviewWindow<tauri::test::MockRuntime>
{
    let config: &WindowConfig = app
        .config()
        .app
        .windows
        .first()
        .expect("the config declares the main window");

    assert_eq!(
        config.label, "main",
        "the capability file names this window"
    );

    WebviewWindowBuilder::from_config(app.handle(), config)
        .expect("the window config should be usable")
        .build()
        .expect("the window should open")
}

fn request(command: &str, body: serde_json::Value) -> InvokeRequest
{
    InvokeRequest {
        cmd: command.to_owned(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: "tauri://localhost".parse().unwrap(),
        body: body.into(),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.to_owned()
    }
}

fn call(command: &str, body: serde_json::Value) -> Result<serde_json::Value, serde_json::Value>
{
    let app = app();
    let window = main_window(&app);

    get_ipc_response(&window, request(command, body))
        .map(|ok| ok.deserialize().expect("a command answers with JSON"))
}

#[test]
fn the_webview_can_ping_the_host()
{
    let answer = call("ping", serde_json::json!({ "id": REQUEST })).expect("ping is allowed");

    assert_eq!(answer["v"], 1);
    assert_eq!(answer["id"], REQUEST);
    assert_eq!(answer["outcome"]["status"], "ok");
    assert_eq!(answer["outcome"]["value"]["protocol_version"], 1);
}

#[test]
fn the_webview_can_ask_for_the_bootstrap_model()
{
    let answer =
        call("bootstrap_get", serde_json::json!({ "id": REQUEST })).expect("bootstrap is allowed");

    assert_eq!(answer["outcome"]["status"], "ok");
    assert_eq!(answer["outcome"]["value"]["protocol_version"], 1);
    assert_eq!(answer["outcome"]["value"]["workspace_open"], false);
}

/// The capability grants no `core:` permission at all, so every one of these is
/// refused.
///
/// Every command listed is a member of `core:default`, which is the one line
/// that would most plausibly be added to the capability file by someone copying
/// a Tauri example. That makes each of these a live canary rather than a
/// decoration: adding `core:default` turns this test red, which was checked by
/// doing it.
#[test]
fn the_webview_cannot_reach_a_core_plugin_command()
{
    let mut reachable = Vec::new();

    for (command, body) in [
        // The window's own title and geometry, from `core:window:default`.
        ("plugin:window|title", serde_json::json!({})),
        ("plugin:window|inner_size", serde_json::json!({})),
        // Path resolution, from `core:path:default` — how a screen would learn
        // where the home directory is without ever being told.
        (
            "plugin:path|resolve_directory",
            serde_json::json!({ "directory": 20 })
        ),
        // Events, from `core:event:default`. The design keeps Rust to UI events
        // for job and process state, and the track that adds them adds the
        // permission with them.
        (
            "plugin:event|listen",
            serde_json::json!({
                "event": "job-state",
                "target": { "kind": "Any" },
                "handler": 0
            })
        ),
        // Enumerating the app's WebViews, from `core:webview:default`.
        ("plugin:webview|get_all_webviews", serde_json::json!({})),
        // The bundle identifier and version, from `core:app:default`.
        ("plugin:app|version", serde_json::json!({})),
        ("plugin:app|identifier", serde_json::json!({}))
    ]
    {
        if call(command, body).is_ok()
        {
            reachable.push(command);
        }
    }

    // Collected rather than asserted one at a time, so a capability that grew
    // reports everything it opened up instead of the first thing alphabetically.
    assert!(
        reachable.is_empty(),
        "these were reachable from the main WebView: {reachable:?}"
    );
}

/// Beyond `core:default`, the commands a second WebView or a window rename would
/// be built out of. None of these is in any default set, so they are refused
/// twice over — but they are the ones whose reachability would undo the
/// per-window capability entirely, so they are named rather than assumed.
#[test]
fn the_webview_cannot_create_another_webview_or_retitle_the_window()
{
    for (command, body) in [
        (
            "plugin:webview|create_webview_window",
            serde_json::json!({ "options": {} })
        ),
        ("plugin:window|create", serde_json::json!({ "options": {} })),
        (
            "plugin:window|set_title",
            serde_json::json!({ "value": "x" })
        ),
        ("plugin:window|close", serde_json::json!({}))
    ]
    {
        assert!(
            call(command, body).is_err(),
            "{command} was reachable from the main WebView"
        );
    }
}

/// A command the app never registered, in the shape a plugin command takes. This
/// is the whole class the design excludes by name: shell, filesystem, SQL.
#[test]
fn an_unregistered_application_command_is_refused()
{
    for command in [
        "plugin:fs|read_text_file",
        "plugin:shell|execute",
        "plugin:sql|select",
        "plugin:dialog|open",
        "plugin:http|fetch",
        "today_get"
    ]
    {
        assert!(
            call(command, serde_json::json!({})).is_err(),
            "{command} answered, and nothing registered it"
        );
    }
}

/// The argument is a validated id, so the parse is part of the boundary rather
/// than part of the handler. A WebView that sends anything else never reaches
/// the function body.
#[test]
fn a_request_id_that_is_not_a_ulid_never_reaches_the_handler()
{
    for id in [
        serde_json::json!("../../etc/passwd"),
        serde_json::json!("01KZNNR5X818P3J6ENYKSADP8"),
        serde_json::json!(""),
        serde_json::json!(7),
        serde_json::json!(null)
    ]
    {
        assert!(
            call("ping", serde_json::json!({ "id": id })).is_err(),
            "{id} was accepted as a request id"
        );
    }
}

/// The capability names one window. A second window — however it came to
/// exist — inherits nothing, because a capability applies to the labels it
/// lists and this one lists `main`.
#[test]
fn a_window_the_capability_does_not_name_can_call_nothing()
{
    let app = app();
    let other = WebviewWindowBuilder::new(
        app.handle(),
        "inspector",
        tauri::WebviewUrl::App("index.html".into())
    )
    .build()
    .expect("the window should open");

    let refused = get_ipc_response(
        &other,
        request("ping", serde_json::json!({ "id": REQUEST }))
    );

    assert!(
        refused.is_err(),
        "a window outside the capability reached a command"
    );
}
