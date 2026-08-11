//! `trdr app status`, from the built binary to a real socket and back.
//!
//! The Tauri app does not exist yet, so what stands in for it is the runtime's
//! own socket server started against a scratch product root. That is the whole
//! path section 9.2 describes apart from who is holding the other end: a real
//! process runs the real binary, which connects to a real Unix socket and reads a
//! real typed answer.

use std::process::{Command, Output};
use std::sync::Arc;
use trdr_core::socket::{AccountInspectResult, AppStatusResult, DataCoverageResult};
use trdr_core::ErrorEnvelope;
use trdr_runtime::clock::SystemClock;
use trdr_runtime::query::{QueryService, SyntheticQueries};
use trdr_runtime::root::ProductRoot;
use trdr_runtime::socket::{AppBridge, ServerHandle, SocketServer};
use trdr_runtime::test_support::scratch_root;

/// The status an app with this workspace open would give.
struct Standing
{
    root: ProductRoot
}

impl AppBridge for Standing
{
    /// The two reads answer from the synthetic query service, the same one the
    /// running app hands its socket bridge.
    fn account_inspect(&self) -> Result<AccountInspectResult, ErrorEnvelope>
    {
        let today = SyntheticQueries::load(SystemClock)?.today()?;

        Ok(AccountInspectResult {
            origin: today.header.origin,
            account: today.account,
            holdings: today.holdings
        })
    }

    fn data_coverage(&self) -> Result<DataCoverageResult, ErrorEnvelope>
    {
        let draft = SyntheticQueries::load(SystemClock)?.lab_draft()?;

        Ok(DataCoverageResult {
            origin: draft.header.origin,
            coverage: draft.coverage
        })
    }
    fn app_status(&self) -> AppStatusResult
    {
        AppStatusResult {
            app_version: "0.0.0".to_owned(),
            pid: std::process::id(),
            holds_writer_lease: true,
            product_root: self.root.path().to_path_buf(),
            workspace_path: self.root.default_workspace_dir(),
            schema_version: 1
        }
    }
}

/// Starts a server standing in for the app.
fn start(root: &ProductRoot) -> ServerHandle
{
    let lease = Arc::new(
        root.acquire_writer_lease()
            .expect("the lease should be free")
    );
    let bridge = Arc::new(Standing { root: root.clone() });

    SocketServer::bind(lease, bridge)
        .expect("the socket should bind")
        .spawn()
}

/// Runs the real `trdr` binary against a product root.
fn trdr(root: &ProductRoot, arguments: &[&str]) -> Output
{
    Command::new(env!("CARGO_BIN_EXE_trdr"))
        .args(arguments)
        .arg("--product-root")
        .arg(root.path())
        .output()
        .expect("the trdr binary could not be run")
}

#[test]
fn app_status_prints_what_the_running_app_said()
{
    let root = scratch_root("cli-status-json");
    let server = start(&root);

    let output = trdr(&root, &["app", "status", "--format", "json"]);

    assert!(
        output.status.success(),
        "trdr app status failed: {output:?}"
    );

    let status: AppStatusResult = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("the output was not a status: {error}"));

    assert_eq!(status.product_root, root.path());
    assert_eq!(status.workspace_path, root.default_workspace_dir());
    assert!(status.holds_writer_lease);
    assert_eq!(status.schema_version, 1);

    server.stop();
}

#[test]
fn the_text_form_names_what_the_app_is_holding()
{
    let root = scratch_root("cli-status-text");
    let server = start(&root);

    let output = trdr(&root, &["app", "status"]);
    let text = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "trdr app status failed: {output:?}"
    );
    assert!(text.contains("running"), "unexpected output: {text}");
    assert!(
        text.contains(&root.default_workspace_dir().display().to_string()),
        "unexpected output: {text}"
    );

    server.stop();
}

// The behaviour stage 0 established, kept: with no app to ask, the answer is the
// envelope, on standard output, and the exit status is 1.
#[test]
fn with_no_app_listening_the_json_form_is_the_app_not_running_envelope()
{
    let root = scratch_root("cli-absent-json");
    let output = trdr(&root, &["app", "status", "--format", "json"]);

    assert_eq!(output.status.code(), Some(1));

    let envelope: ErrorEnvelope = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("the output was not an envelope: {error}"));

    assert_eq!(envelope.code, trdr_core::ErrorCode::AppNotRunning);
}

#[test]
fn with_no_app_listening_the_text_form_goes_to_standard_error()
{
    let root = scratch_root("cli-absent-text");
    let output = trdr(&root, &["app", "status"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "a failure was written to stdout");
    assert!(String::from_utf8_lossy(&output.stderr).contains("APP_NOT_RUNNING"));
}

// The app stopping is not a different answer from the app never having been
// there, which is what makes the socket file irrelevant to the question.
#[test]
fn an_app_that_stops_goes_back_to_app_not_running()
{
    let root = scratch_root("cli-stopped");
    let server = start(&root);

    assert!(trdr(&root, &["app", "status"]).status.success());

    server.stop();

    let output = trdr(&root, &["app", "status", "--format", "json"]);
    let envelope: ErrorEnvelope = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(envelope.code, trdr_core::ErrorCode::AppNotRunning);
}

/// The proof milestone M2 asks for: the CLI reads the same domain object the
/// screen does.
///
/// The server here answers `account.inspect` from the same
/// [`SyntheticQueries`] the running app hands both its socket bridge and its
/// Tauri commands, so a value that reached this output reached the screen from
/// one place. What is checked is the round trip and the shape — the CLI receives
/// a typed `AccountInspectResult`, not a blob it has to interpret.
#[test]
fn account_inspect_returns_the_same_model_the_screen_renders()
{
    let root = scratch_root("cli-account-json");
    let server = start(&root);

    let output = trdr(&root, &["account", "inspect", "--format", "json"]);

    assert!(
        output.status.success(),
        "trdr account inspect failed: {output:?}"
    );

    let account: AccountInspectResult = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("the output was not an account: {error}"));
    let expected = SyntheticQueries::load(SystemClock)
        .expect("the fixture parses")
        .today()
        .expect("the model builds");

    assert_eq!(account.origin, expected.header.origin);
    assert_eq!(account.account, expected.account);
    assert_eq!(account.holdings, expected.holdings);

    server.stop();
}

/// Synthetic output says so, and says so first.
///
/// Terminal output outlives the window it was produced beside — it gets
/// scrolled past, pasted into a message, read back the next day. A number that
/// arrives with no statement of where it came from is the failure mode
/// milestone M2's rule exists to prevent, so the banner is asserted rather than
/// left to a reviewer noticing it.
#[test]
fn synthetic_output_says_it_is_synthetic_before_it_says_anything_else()
{
    let root = scratch_root("cli-account-text");
    let server = start(&root);

    let account = trdr(&root, &["account", "inspect"]);
    let coverage = trdr(&root, &["data", "coverage"]);

    for output in [&account, &coverage]
    {
        let text = String::from_utf8_lossy(&output.stdout);

        assert!(output.status.success(), "the command failed: {output:?}");
        assert!(
            text.lines()
                .next()
                .is_some_and(|line| line.contains("synthetic")),
            "the first line did not say the data is synthetic: {text}"
        );
    }

    server.stop();
}

/// The gaps reach the person as dates rather than as a count, because a count
/// cannot be acted on.
#[test]
fn data_coverage_names_the_days_that_are_missing()
{
    let root = scratch_root("cli-coverage-text");
    let server = start(&root);

    let output = trdr(&root, &["data", "coverage"]);
    let text = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "trdr data coverage failed: {output:?}"
    );
    assert!(text.contains("missing"), "unexpected output: {text}");
    assert!(text.contains("2026-06-15"), "unexpected output: {text}");

    server.stop();
}
