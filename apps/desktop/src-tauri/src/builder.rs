//! The one place the command list is written down.
//!
//! Tauri's dispatch table and the TypeScript the WebView imports both come out
//! of [`commands`]. Registering a command in one and not the other is what makes
//! a "typed" bridge untyped in exactly one spot, so there is no second list to
//! forget: `collect_commands!` feeds the handler and the exporter alike.

use tauri_specta::{collect_commands, Builder};

/// The commands the main WebView may call.
///
/// Adding one here is three edits, and the tests refuse the first two on their
/// own: the handler goes in [`crate::commands::COMMANDS`], its name goes in
/// `build.rs` so Tauri generates a permission for it, and that permission goes
/// in `capabilities/main.json` so the main window is actually allowed to call
/// it. A command registered here and left out of the capability file is
/// reachable from nowhere; a command left out of `build.rs` is reachable from
/// everywhere.
///
/// Generic over the runtime so that the tests can build the same command list
/// against Tauri's mock runtime. A test that registered its own list would prove
/// something about the test and nothing about the app.
pub fn commands<R: tauri::Runtime>() -> Builder<R>
{
    Builder::<R>::new().commands(collect_commands![
        crate::commands::ping,
        crate::commands::bootstrap_get
    ])
}
