//! The trdr desktop host.
//!
//! This crate is the composition root of the application half of trdr: it owns
//! the main window, the capability file that says what that window may call, and
//! the typed commands behind it. It owns no domain logic — that is
//! `trdr-core` — and no effects — those are `trdr-runtime`'s.
//!
//! # What this build does not do
//!
//! Starting it opens a window and nothing else. It does not create or read
//! `~/.trdr`, open a database, take a writer lease, or listen on a socket. The
//! commands it registers answer from constants. Those capabilities arrive with
//! the tracks that own them, and the seam they arrive through is
//! [`commands`] — the handlers there are what get a real implementation, not the
//! window and not the capability file.
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

/// Builds the application, ready to run or to drive from a test.
///
/// The window itself comes from `tauri.conf.json`, so a test that builds this
/// gets the same window label and the same capability file the shipped app does.
pub fn app() -> tauri::Builder<tauri::Wry>
{
    tauri::Builder::default().invoke_handler(builder::commands::<tauri::Wry>().invoke_handler())
}

/// Runs the desktop app.
///
/// # Panics
///
/// If the window cannot be created, which is not a condition the app can
/// meaningfully continue past.
pub fn run()
{
    app()
        .run(tauri::generate_context!())
        .expect("failed to start the trdr desktop app");
}
