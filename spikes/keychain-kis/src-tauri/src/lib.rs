pub mod credential;
pub mod http;
pub mod kis;
pub mod logging;
pub mod mock;
pub mod scan;
pub mod scenario;
pub mod secret;
pub mod sheet;
pub mod store;
pub mod workspace;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::NSWindow;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use credential::{CredentialState, CredentialStore, KeychainStore};
use logging::Log;
use scan::{Needle, Report};
use scenario::{Canaries, Outcome};
use workspace::Workspace;

struct Spike
{
    workspace: Workspace,
    instance_id: String,
    log: Log,
    canaries: Canaries,
    keychain: KeychainStore,
    sheet_step: Mutex<Option<String>>,
    outcome: Mutex<Option<Outcome>>,
    report: Mutex<Option<Report>>
}

/// Everything the WebView is allowed to know.
///
/// One serializable value with no [`secret::Secret`] anywhere in its type, so the
/// compiler is what stops a credential reaching the page rather than a reviewer
/// noticing that it did.
#[derive(Clone, Serialize)]
struct SpikeView
{
    workspace: String,
    instance_id: String,
    sheet_step: Option<String>,
    outcome: Option<Outcome>,
    report: Option<Report>
}

#[tauri::command]
fn spike_state(spike: State<Arc<Spike>>) -> SpikeView
{
    SpikeView
    {
        workspace: spike.workspace.root().display().to_string(),
        instance_id: spike.instance_id.clone(),
        sheet_step: spike.sheet_step.lock().unwrap().clone(),
        outcome: spike.outcome.lock().unwrap().clone(),
        report: spike.report.lock().unwrap().clone()
    }
}

fn main_ns_window(app: &AppHandle) -> Option<Retained<NSWindow>>
{
    let window = app.get_webview_window("main")?;
    let pointer = window.ns_window().ok()?;
    unsafe { Retained::retain(pointer.cast::<NSWindow>()) }
}

/// Raises the credential sheet and answers it, without a person.
///
/// The sheet is the surface section 10 puts a credential behind, so it has to be
/// the way the value gets in. Reaching around it and writing to the Keychain
/// directly would leave the path this spike exists to prove untested.
fn drive_sheet(app: &AppHandle, spike: &Arc<Spike>)
{
    let saving = Arc::clone(spike);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move ||
    {
        let Some(mtm) = MainThreadMarker::new() else { return };
        let Some(parent) = main_ns_window(&handle) else { return };
        sheet::present(mtm, &parent, Box::new(move |app_key, app_secret|
        {
            // Straight into the Keychain from the sheet's own handler. The two
            // values do not leave this closure.
            let key_account = credential::account(&saving.instance_id, credential::COLLECTOR, "app_key");
            let secret_account = credential::account(&saving.instance_id, credential::COLLECTOR, "app_secret");
            let stored = saving.keychain.set(&key_account, &app_key).is_ok()
                && saving.keychain.set(&secret_account, &app_secret).is_ok();

            let code = if stored { "AUTH_STORED" } else { "AUTH_STORE_REFUSED" };
            saving.log.event(code, &[("step", "credential.sheet"), ("collector", credential::COLLECTOR)]);
            *saving.sheet_step.lock().unwrap() = Some(code.to_string());
        }));
    });

    std::thread::sleep(Duration::from_millis(900));

    let typed = spike.canaries.app_secret.clone();
    let _ = app.run_on_main_thread(move ||
    {
        sheet::fill_and_click(scenario::APP_KEY, &typed, true);
    });
    std::thread::sleep(Duration::from_millis(600));
}

fn drive(app: AppHandle, spike: Arc<Spike>)
{
    // The window has to exist before a sheet can hang off it.
    std::thread::sleep(Duration::from_millis(1200));
    drive_sheet(&app, &spike);

    let after_sheet = spike
        .keychain
        .state(&credential::account(&spike.instance_id, credential::COLLECTOR, "app_secret"));
    if after_sheet != CredentialState::Untested
    {
        spike.log.event("AUTH_MISSING", &[("step", "credential.sheet_readback"), ("collector", credential::COLLECTOR)]);
    }

    let outcome = scenario::run(&spike.workspace, &spike.instance_id, &spike.keychain, &spike.log, &spike.canaries);
    *spike.outcome.lock().unwrap() = Some(outcome);

    let mut artifacts = spike.workspace.artifacts();
    artifacts.extend(scan::crash_reports("trdr-spike-keychain-kis"));
    let report = scan::scan(&artifacts, &needles(&spike.canaries));
    let clean = report.clean();
    *spike.report.lock().unwrap() = Some(report);

    spike.log.event(if clean { "SCAN_CLEAN" } else { "SCAN_LEAKED" }, &[("step", "scan")]);
    let _ = app.emit("spike://done", clean);
}

fn needles(canaries: &Canaries) -> Vec<Needle>
{
    vec![
        Needle::new("app secret", &canaries.app_secret),
        Needle::new("account number", &canaries.account_no),
        // Not a canary the spike planted, but a secret the mock minted. It must
        // not have been persisted either.
        Needle::new("access token", "mock-access-token")
    ]
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run()
{
    tauri::Builder::default()
        .setup(|app|
        {
            let root = workspace::product_root();
            let workspace = Workspace::open(&root)?;
            let spike = Arc::new(Spike
            {
                log: Log::new(&workspace.log_path()),
                instance_id: workspace::local_instance_id(&root)?,
                workspace,
                canaries: Canaries::default(),
                keychain: KeychainStore,
                sheet_step: Mutex::new(None),
                outcome: Mutex::new(None),
                report: Mutex::new(None)
            });

            app.manage(Arc::clone(&spike));
            let handle = app.handle().clone();
            std::thread::spawn(move || drive(handle, spike));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![spike_state])
        .run(tauri::generate_context!())
        .expect("the spike window failed to start");
}
