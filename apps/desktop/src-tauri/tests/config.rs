//! The settings Tauri and Vite each hold half of.
//!
//! Two configuration files describe one dev server and one build output, and
//! neither can see the other. When they disagree the result is not a build
//! failure — it is a window that comes up blank, or a packaged app that ships
//! the previous build's assets. Both are found by hand, late.
//!
//! So the pairs are asserted here. The Rust side reads `tauri.conf.json` through
//! the same types Tauri does, and the Vite side is read as text: `vite.config.ts`
//! is TypeScript and nothing in a Rust test can evaluate it, but the values in
//! question are literals and matching on them is enough to notice a change.

use std::path::{Path, PathBuf};
use tauri::utils::config::{Config, FrontendDist};

/// The dev server port, written down once here and checked against both files.
const DEV_PORT: u16 = 1420;

fn desktop_directory() -> PathBuf
{
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("src-tauri sits inside the desktop app directory")
        .to_path_buf()
}

fn config() -> Config
{
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tauri.conf.json");
    let text = std::fs::read_to_string(path).expect("tauri.conf.json should be readable");
    serde_json::from_str(&text).expect("tauri.conf.json should parse as a Tauri config")
}

fn vite_config() -> String
{
    std::fs::read_to_string(desktop_directory().join("vite.config.ts"))
        .expect("vite.config.ts should be readable")
}

#[test]
fn the_dev_url_is_the_port_vite_is_pinned_to()
{
    let url = config().build.dev_url.expect("a devUrl is configured");

    assert_eq!(url.port(), Some(DEV_PORT));
    assert_eq!(url.host_str(), Some("localhost"));

    let vite = vite_config();
    assert!(
        vite.contains(&format!("port: {DEV_PORT}")),
        "vite.config.ts does not pin port {DEV_PORT}"
    );

    // Without `strictPort`, Vite moves to the next free port when this one is
    // taken and says so in a line nobody reads, and the window then points at a
    // server that is not there.
    assert!(
        vite.contains("strictPort: true"),
        "vite.config.ts lets the dev server move off its port"
    );
}

#[test]
fn the_frontend_dist_is_where_vite_writes()
{
    let config = config();

    let Some(FrontendDist::Directory(dist)) = config.build.frontend_dist
    else
    {
        panic!("frontendDist should be a directory");
    };

    let resolved = Path::new(env!("CARGO_MANIFEST_DIR")).join(&dist);
    let expected = desktop_directory().join("dist");

    assert_eq!(
        resolved.canonicalize().ok(),
        expected.canonicalize().ok(),
        "frontendDist ({}) is not where vite.config.ts writes ({})",
        resolved.display(),
        expected.display()
    );
    assert!(
        vite_config().contains("outDir: \"dist\""),
        "vite.config.ts does not write to `dist`"
    );
}

/// The identifier is a settled decision (`01kznp0gq3x8ark9dq489z7wj8`), and it is
/// also the Keychain service name, so changing it silently orphans every stored
/// credential on a user's machine.
#[test]
fn the_bundle_identifier_is_the_settled_one()
{
    assert_eq!(config().identifier, "com.fxylabs.trdr");
}

/// A content security policy that has quietly become permissive is not visible
/// in a running app. These are the directives whose absence would matter.
#[test]
fn the_content_security_policy_stays_restrictive()
{
    let config = config();
    let csp = config
        .app
        .security
        .csp
        .expect("a CSP is configured")
        .to_string();

    for directive in [
        // Nothing loads unless a later directive names it.
        "default-src 'none'",
        // No inline script, which is what makes anything injected into a screen
        // inert rather than executable.
        "script-src 'self'",
        // No outbound network from the WebView. The host does the network, and
        // section 9.1 is the only way to ask it to.
        "connect-src 'self' ipc: http://ipc.localhost",
        "object-src 'none'",
        "frame-ancestors 'none'",
        "base-uri 'none'",
        "form-action 'none'"
    ]
    {
        assert!(
            csp.contains(directive),
            "the CSP no longer says {directive}"
        );
    }

    assert!(
        !csp.contains("script-src 'self' 'unsafe-inline'"),
        "the shipped CSP allows inline script"
    );
    assert!(!csp.contains("unsafe-eval"), "the shipped CSP allows eval");

    // The dev policy is looser — Vite injects its client inline and talks over a
    // websocket — but it is a separate string precisely so that looseness cannot
    // reach a packaged build.
    let dev = config
        .app
        .security
        .dev_csp
        .expect("a dev CSP is configured")
        .to_string();

    assert!(dev.contains(&format!("ws://localhost:{DEV_PORT}")));
    assert!(!dev.contains("unsafe-eval"), "the dev CSP allows eval");
}

/// `withGlobalTauri` puts the whole API object on `window`. It is off, so the
/// only route to the host is the generated bindings, which import `invoke`
/// directly.
#[test]
fn the_api_is_not_published_on_the_window_object()
{
    assert!(!config().app.with_global_tauri);
}

/// The capability file grants the registered commands, and grants nothing else.
///
/// `tests/capability.rs` proves what is reachable by reaching for it, which is
/// the stronger check but can only ever cover the commands someone thought to
/// name. This one reads the file and compares it against the command list, so an
/// entry nobody anticipated — a plugin, a `core:` set, a permission for a
/// command that was deleted — fails without having to have been predicted.
#[test]
fn the_capability_grants_the_registered_commands_and_nothing_else()
{
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities/main.json");
    let text = std::fs::read_to_string(path).expect("the capability file should be readable");
    let capability: serde_json::Value =
        serde_json::from_str(&text).expect("the capability file should be JSON");

    let granted: Vec<&str> = capability["permissions"]
        .as_array()
        .expect("permissions is a list")
        .iter()
        .map(|value| value.as_str().expect("a permission is a string"))
        .collect();

    let expected: Vec<&str> = trdr_desktop_lib::commands::COMMANDS
        .iter()
        .map(|command| command.permission)
        .collect();

    assert_eq!(granted, expected);

    // The capability applies to the labels it lists. One window, named once.
    assert_eq!(
        capability["windows"].as_array().map(Vec::as_slice),
        Some([serde_json::Value::from("main")].as_slice())
    );
}
