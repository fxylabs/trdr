//! Declares the app's own commands to Tauri's access-control list.
//!
//! This is the load-bearing part of `docs/FOUNDATION_DESIGN.md` section 9.1's
//! rule that the WebView reaches only commands the app wrote. Naming the
//! commands here makes Tauri generate an application ACL manifest, and the
//! presence of that manifest flips app commands from "always callable" to
//! "callable only where a capability file allows them". Without this call the
//! capability file would describe the plugin surface and say nothing at all
//! about `ping` or `bootstrap_get`, and both would be reachable from any
//! WebView the app ever opens.
//!
//! The list must stay in step with `commands::COMMANDS` and with the permissions
//! in `capabilities/main.json`; `tests/capability.rs` is what checks that it
//! does.

fn main()
{
    let attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "ping",
            "bootstrap_get",
            "terminal_start",
            "terminal_input",
            "terminal_resize",
            "terminal_restart"
        ]));

    tauri_build::try_build(attributes).expect("failed to run the Tauri build script");
}
