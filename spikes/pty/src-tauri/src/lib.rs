mod env;
mod pty;
mod scrollback;

use base64::Engine as _;
use tauri::{AppHandle, Emitter, State};

use pty::{PtyState, SpawnInfo, SpawnRequest};

#[tauri::command]
fn env_report() -> env::EnvReport
{
    env::report()
}

#[tauri::command]
fn pty_spawn(app: AppHandle, state: State<PtyState>, request: SpawnRequest) -> Result<SpawnInfo, String>
{
    let (session, info) = pty::open(&request, output_sink(app.clone()), closed_sink(app))?;
    pty::hold(&state, session)?;
    Ok(info)
}

// The only two things that happen to a byte the host read: it is persisted as
// itself, and it is base64'd across to the WebView. Neither step looks at it.
fn output_sink(app: AppHandle) -> pty::OutputSink
{
    Box::new(move |bytes: &[u8]|
    {
        if let Some(file) = scrollback::default_path()
        {
            scrollback::append(&file, bytes);
        }
        let _ = app.emit("pty://output", encode(bytes));
    })
}

fn closed_sink(app: AppHandle) -> pty::ClosedSink
{
    Box::new(move || { let _ = app.emit("pty://closed", ()); })
}

fn encode(bytes: &[u8]) -> String
{
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[tauri::command]
fn pty_write(state: State<PtyState>, data: String) -> Result<(), String>
{
    pty::write(&state, &data)
}

#[tauri::command]
fn pty_resize(state: State<PtyState>, cols: u16, rows: u16) -> Result<(), String>
{
    pty::resize(&state, cols, rows)
}

#[tauri::command]
fn pty_kill(state: State<PtyState>) -> Result<(), String>
{
    pty::kill(&state)
}

#[tauri::command]
fn pty_running(state: State<PtyState>) -> bool
{
    pty::running(&state)
}

#[tauri::command]
fn scrollback_load() -> String
{
    let bytes = scrollback::default_path().map(|file| scrollback::load(&file)).unwrap_or_default();
    encode(&bytes)
}

#[tauri::command]
fn scrollback_clear()
{
    if let Some(file) = scrollback::default_path()
    {
        scrollback::clear(&file);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run()
{
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(PtyState::default())
        .invoke_handler(tauri::generate_handler![
            env_report,
            pty_spawn,
            pty_write,
            pty_resize,
            pty_kill,
            pty_running,
            scrollback_load,
            scrollback_clear
        ])
        .run(tauri::generate_context!())
        .expect("the spike window failed to start");
}
