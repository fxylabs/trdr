//! What actually happens when the app starts, against a real product root.
//!
//! Section 14's stop condition is about four things being true at once: the app
//! and the CLI find the same workspace and runtime, the app holds the single
//! writer lease, the socket answers, and a second copy cannot. The other test
//! files in this crate cover the window and the command surface with nothing
//! behind them; this one runs [`AppRuntime::start`] for real — an `flock`, a
//! SQLite file with migration 0 in it, a `workspace.json`, and a bound Unix
//! socket — and then asks the runtime's own client the same question `trdr app
//! status` asks.
//!
//! Every root here comes from [`scratch_root`], which lives under `/private/tmp`
//! and asserts it is not inside the home directory. Nothing in this file, or
//! anywhere else in this crate's tests, can reach the `~/.trdr` a person is
//! using: [`ProductRoot::for_current_user`] is called from exactly one place in
//! the app, and [`the_only_route_to_the_real_product_root_is_the_app_itself`]
//! is what keeps that true.

use std::path::Path;
use trdr_core::workspace::WorkspaceManifest;
use trdr_desktop_lib::startup::AppRuntime;
use trdr_runtime::root::{mode_of, ProductRoot, PRIVATE_FILE_MODE};
use trdr_runtime::socket::AppClient;
use trdr_runtime::test_support::scratch_root;

/// What the app reports about itself, read the way the CLI reads it.
fn status_over_the_socket(root: &ProductRoot) -> trdr_core::socket::AppStatusResult
{
    AppClient::connect(root)
        .expect("the app should be listening")
        .app_status()
        .expect("the app should answer")
}

#[test]
fn a_first_run_leaves_a_workspace_a_database_and_a_socket()
{
    let root = scratch_root("app-first-run");
    let app = AppRuntime::start(root.clone()).expect("a first run should come up");

    let manifest_path = root.default_workspace_dir().join("workspace.json");
    let manifest = WorkspaceManifest::from_json(
        &std::fs::read_to_string(&manifest_path).expect("the manifest should be there")
    )
    .expect("the manifest should be readable");

    assert_eq!(manifest.workspace_id, app.workspace().id());
    assert_eq!(mode_of(&manifest_path).unwrap(), PRIVATE_FILE_MODE);
    assert!(root.default_database_path().exists());
    assert!(root.socket_path().exists());

    app.shut_down();
}

/// The stop condition in one assertion: what the app thinks it has open is what
/// a caller on the socket is told, and both are the root the app was started
/// against.
#[test]
fn the_app_and_a_caller_on_the_socket_find_the_same_workspace()
{
    let root = scratch_root("app-same-workspace");
    let app = AppRuntime::start(root.clone()).expect("it should come up");

    let over_the_socket = status_over_the_socket(&root);

    assert_eq!(over_the_socket, app.status());
    assert_eq!(over_the_socket.product_root, root.path());
    assert_eq!(over_the_socket.workspace_path, root.default_workspace_dir());
    assert_eq!(over_the_socket.pid, std::process::id());
    assert!(over_the_socket.holds_writer_lease);
    assert_eq!(over_the_socket.schema_version, 1);

    app.shut_down();
}

/// `bootstrap.get`'s answer and `app.status`'s answer are produced by different
/// code on different sides of the process, and a screen that disagreed with the
/// CLI about which workspace is open would be the confusing kind of wrong.
#[test]
fn the_screen_and_the_cli_are_told_the_same_workspace()
{
    let root = scratch_root("app-agree");
    let app = AppRuntime::start(root.clone()).expect("it should come up");

    let bootstrap = app.bootstrap().clone();
    let status = status_over_the_socket(&root);

    assert_eq!(bootstrap.workspace_path, status.workspace_path);
    assert_eq!(i64::from(bootstrap.schema_version), status.schema_version);
    assert_eq!(bootstrap.workspace_id, app.workspace().id());

    app.shut_down();
}

/// The single writer lease, which is what makes any of the above safe.
#[test]
fn a_second_app_cannot_start_beside_the_first()
{
    let root = scratch_root("app-single-writer");
    let first = AppRuntime::start(root.clone()).expect("the first should come up");

    let Err(refused) = AppRuntime::start(root.clone())
    else
    {
        panic!("two apps took the writer lease at once");
    };
    assert!(refused.is_lease_held());

    // And the first is still serving: a refused second start must not have
    // unlinked the socket or disturbed the lease on its way out.
    assert!(status_over_the_socket(&root).holds_writer_lease);

    // Dropping the runtime, not shutting it down: `shut_down` stops the socket
    // and leaves the lease to the process ending, which in a test is never.
    drop(first);

    let next = AppRuntime::start(root).expect("the lease should be free once the first has gone");
    assert!(next.status().holds_writer_lease);
}

/// A refused start must leave nothing behind. If it did, the failure would be
/// sticky: the app would refuse to start from then on, with no process holding
/// anything.
#[test]
fn a_refused_start_releases_everything_it_took()
{
    let root = scratch_root("app-clean-refusal");
    let holder = AppRuntime::start(root.clone()).expect("the first should come up");

    for _ in 0..3
    {
        assert!(AppRuntime::start(root.clone()).is_err());
    }

    drop(holder);

    let after = AppRuntime::start(root).expect("three refusals should have left no residue");
    assert!(after.status().holds_writer_lease);
}

/// The crash case. A process killed outright leaves its socket file behind, and
/// the kernel drops its `flock` — so the next start finds a file it must clear
/// and a lease it may take. Simulated here by leaving the file in place, which
/// is exactly the state a `SIGKILL` leaves the directory in.
#[test]
fn a_socket_file_left_by_a_dead_app_does_not_stop_the_next_one()
{
    let root = scratch_root("app-stale-socket");
    let crashed = AppRuntime::start(root.clone()).expect("the first should come up");
    let socket = root.socket_path();

    // Drop the runtime without shutting it down, then put the file back: the
    // combination is what a killed process leaves, since its destructors never
    // ran but its descriptors were closed by the kernel.
    drop(crashed);
    std::fs::write(&socket, b"").expect("a leftover socket file could not be staged");
    assert!(socket.exists());

    let next = AppRuntime::start(root.clone()).expect("a stale socket should not block a start");

    assert!(status_over_the_socket(&root).holds_writer_lease);
    next.shut_down();
}

/// Both ways out remove the socket file: the explicit one Tauri's exit event
/// takes, and the destructor, for anything that drops the runtime instead.
#[test]
fn either_way_out_removes_the_socket_file()
{
    let root = scratch_root("app-shutdown");
    let app = AppRuntime::start(root.clone()).expect("it should come up");
    assert!(root.socket_path().exists());

    app.shut_down();
    assert!(
        !root.socket_path().exists(),
        "the socket outlived the app that bound it"
    );

    // Twice is not a failure: `RunEvent::Exit` is not the only path there.
    app.shut_down();
    drop(app);

    // What a caller sees afterwards is the ordinary "no app is running" case.
    assert!(AppClient::connect(&root).is_err());

    let dropped = AppRuntime::start(root.clone()).expect("the lease is free again");
    assert!(root.socket_path().exists());
    drop(dropped);
    assert!(
        !root.socket_path().exists(),
        "a dropped app kept its socket"
    );
}

/// The guard the brief asks for, as a test rather than a habit.
///
/// `~/.trdr` is named in exactly one place in this crate — the fallback in
/// `startup::product_root`, which is reached only when `TRDR_PRODUCT_ROOT` is
/// unset — and no test calls it. A handler that started resolving its own root,
/// or a test helper that reached for `for_current_user`, would show up here as a
/// second mention.
#[test]
fn the_only_route_to_the_real_product_root_is_the_app_itself()
{
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut mentions = Vec::new();

    let mut pending = vec![source.clone()];

    while let Some(directory) = pending.pop()
    {
        for entry in std::fs::read_dir(&directory).expect("the source tree should be readable")
        {
            let path = entry.expect("a directory entry").path();

            if path.is_dir()
            {
                pending.push(path);
                continue;
            }

            if path.extension().is_some_and(|kind| kind == "rs")
                && std::fs::read_to_string(&path)
                    .unwrap_or_default()
                    .contains("for_current_user")
            {
                mentions.push(path);
            }
        }
    }

    assert_eq!(
        mentions,
        [source.join("startup.rs")],
        "the real product root is reachable from somewhere new"
    );
}
