//! The trdr desktop host.
//!
//! This crate is the composition root of the application half of trdr: it owns
//! the main window, the capability file that says what that window may call, and
//! the typed commands behind it. It owns no domain logic — that is
//! `trdr-core` — and no effects — those are `trdr-runtime`'s.
//!
//! # What starting it does
//!
//! [`startup`] brings the runtime up before any window exists: it resolves the
//! product root, takes the single writer lease, opens the workspace database and
//! runs migration 0 if the file is new, creates `workspace.json` on a first run,
//! and binds the Unix socket the `trdr` CLI talks to. Only then is a window
//! created. A failure at any of those steps is written to standard error as a
//! sentence and an error envelope, and the process exits without a window — see
//! [`startup`] for why that is the whole of the second-instance behaviour.
//!
//! # How the WebView is confined
//!
//! `build.rs` declares this crate's commands to Tauri's access-control list.
//! That declaration is what makes application commands subject to the ACL at
//! all: with no application manifest, Tauri treats every `#[tauri::command]` as
//! callable from any WebView, and the capability file would only ever have been
//! describing plugins. With it, `capabilities/main.json` is the complete answer
//! to what the main window can reach, and it lists two commands.
//!
//! No Tauri plugin is linked into this binary. There is no filesystem, shell,
//! SQL, HTTP, dialog, or clipboard command to expose, so `docs/FOUNDATION_DESIGN.md`
//! section 9.1's rule about generic plugin commands is held by the dependency
//! list and not only by the capability file.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod bindings;
pub mod builder;
pub mod commands;
pub mod startup;

/// Builds the application, ready to run or to drive from a test.
///
/// The window itself comes from `tauri.conf.json`, so a test that builds this
/// gets the same window label and the same capability file the shipped app does.
pub fn app() -> tauri::Builder<tauri::Wry>
{
    tauri::Builder::default().invoke_handler(builder::commands::<tauri::Wry>().invoke_handler())
}

/// Runs the desktop app: runtime first, then the window.
///
/// # Panics
///
/// If the window cannot be created. Every condition the app can anticipate is
/// handled before this point and exits without a panic; a window that cannot be
/// created is the operating system refusing, and there is nothing left to do.
pub fn run()
{
    let runtime = match startup::product_root().and_then(startup::AppRuntime::start)
    {
        Ok(runtime) => runtime,
        Err(error) => startup::report_and_exit(&error)
    };

    app()
        // Two managed values from one: the commands read the small settled facts
        // and never see the lease, the connection, or the socket.
        .manage(runtime.bootstrap().clone())
        .manage(runtime)
        .build(tauri::generate_context!())
        .expect("failed to start the trdr desktop app")
        .run(|handle, event| {
            // Not left to `Drop`. The event loop underneath Tauri ends the
            // process on some paths, and a destructor that does not run is a
            // socket file that outlives the app that bound it.
            if matches!(event, tauri::RunEvent::Exit)
            {
                use tauri::Manager as _;
                handle.state::<startup::AppRuntime>().shut_down();
            }
        });
}
