pub mod approval;
pub mod peer;
pub mod protocol;
pub mod runtime;
pub mod server;
mod sheet;

use std::sync::Arc;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::NSWindow;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use approval::{ApprovalSummary, Broker, Outcome};
use protocol::PROTOCOL_VERSION;
use server::{Bridge, Host};

#[derive(Clone, Serialize)]
struct BridgeState
{
    listening: bool,
    socket_path: String,
    lease_path: String,
    protocol_version: u32,
    problem: Option<String>
}

struct BridgeRuntime
{
    state: BridgeState,
    // The lease is held for as long as the app runs. Dropping it would hand the
    // writer role to another process while this one is still serving.
    _lease: Option<runtime::WriterLease>
}

#[tauri::command]
fn bridge_state(runtime: State<BridgeRuntime>) -> BridgeState
{
    runtime.state.clone()
}

struct TauriHost
{
    app: AppHandle,
    broker: Arc<Broker>
}

impl Host for TauriHost
{
    fn open_ui(&self, resource: &str, id: &str) -> Result<(), String>
    {
        let window = self.app.get_webview_window("main").ok_or("the spike window is not open")?;
        let _ = window.unminimize();
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
        self.log(format!("ui.open {resource} {id}"));
        Ok(())
    }

    /// The pitfall spike 1 handed over: AppKit may only be touched on the main
    /// thread, and Tauri's event loop owns it. The socket server answers on a
    /// connection thread, so the sheet cannot be raised where the request was
    /// read — it has to be handed across.
    fn present_approval(&self, summary: ApprovalSummary)
    {
        let request_id = summary.request_id.clone();
        let answering_id = request_id.clone();
        let app = self.app.clone();
        let broker = Arc::clone(&self.broker);

        let dispatched = self.app.run_on_main_thread(move ||
        {
            let Some(mtm) = MainThreadMarker::new() else { return };
            let Some(parent) = main_ns_window(&app) else { return };
            sheet::present(mtm, &parent, &summary, Box::new(move |approved|
            {
                let wanted = if approved { Outcome::Approved } else { Outcome::Rejected };
                broker.settle(&answering_id, wanted);
            }));
        });

        // A sheet that never reached the main thread would leave the caller
        // waiting until its own expiry, which reads as an unresponsive app. Say
        // so and settle it now.
        if dispatched.is_err()
        {
            self.log("the approval sheet could not reach the main thread".to_string());
            self.broker.settle(&request_id, Outcome::Cancelled);
        }
    }

    fn dismiss_approval(&self, request_id: &str)
    {
        let request_id = request_id.to_string();
        let _ = self.app.run_on_main_thread(move || sheet::dismiss(&request_id));
    }

    fn click_sheet(&self, approve: bool) -> Result<(), String>
    {
        self.app
            .run_on_main_thread(move || { sheet::click(approve); })
            .map_err(|error| error.to_string())
    }

    fn log(&self, line: String)
    {
        let _ = self.app.emit("bridge://log", line);
    }
}

fn main_ns_window(app: &AppHandle) -> Option<Retained<NSWindow>>
{
    let window = app.get_webview_window("main")?;
    let pointer = window.ns_window().ok()?;
    // Tauri owns this window; retaining it keeps it alive for as long as the
    // sheet is attached to it.
    unsafe { Retained::retain(pointer.cast::<NSWindow>()) }
}

fn start_bridge(app: &AppHandle) -> BridgeRuntime
{
    let socket_path = runtime::socket_path().map(|path| path.display().to_string()).unwrap_or_default();
    let lease_path = runtime::lease_path().map(|path| path.display().to_string()).unwrap_or_default();

    let started = runtime::run_dir().and_then(|run| server::bind(&run));
    let (lease, listener) = match started
    {
        Ok(pair) => pair,
        Err(error) => return BridgeRuntime
        {
            state: BridgeState
            {
                listening: false,
                socket_path,
                lease_path,
                protocol_version: PROTOCOL_VERSION,
                problem: Some(error.to_string())
            },
            _lease: None
        }
    };

    let broker = Arc::new(Broker::default());
    let host = Arc::new(TauriHost { app: app.clone(), broker: Arc::clone(&broker) });
    let bridge = Arc::new(Bridge { broker, host });
    std::thread::spawn(move || bridge.serve(listener));

    BridgeRuntime
    {
        state: BridgeState
        {
            listening: true,
            socket_path,
            lease_path,
            protocol_version: PROTOCOL_VERSION,
            problem: None
        },
        _lease: Some(lease)
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run()
{
    tauri::Builder::default()
        .setup(|app|
        {
            let bridge = start_bridge(app.handle());
            app.manage(bridge);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![bridge_state])
        .run(tauri::generate_context!())
        .expect("the spike window failed to start");
}
