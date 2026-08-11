//! The socket server, over a real socket, against a scratch product root.
//!
//! What these cover that the unit tests in `src/socket.rs` cannot: binding a real
//! path, the stale socket file a dead app leaves behind, and the exclusion
//! between a server that is running and one that tries to start beside it.

use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;
use std::time::Duration;
use trdr_core::error::ErrorEnvelope;
use trdr_core::socket::{AccountInspectResult, DataCoverageResult};
use trdr_core::socket::{
    AppStatusResult, SocketMethod, SocketOutcome, SocketRequest, SocketResponse
};
use trdr_core::ErrorCode;
use trdr_runtime::clock::SystemClock;
use trdr_runtime::query::{QueryService, SyntheticQueries};
use trdr_runtime::root::{mode_of, ProductRoot, PRIVATE_FILE_MODE};
use trdr_runtime::socket::{
    AppBridge, AppClient, ClientError, ServerHandle, SocketServer, REQUEST_READ_TIMEOUT
};
use trdr_runtime::test_support::scratch_root;

/// A bridge that answers with a fixed status.
struct Fixed;

impl AppBridge for Fixed
{
    /// The two reads answer from the synthetic query service, which is also what
    /// the running app hands its socket bridge. A hand-built value here would
    /// prove the frame carried something; this proves it carried the model.
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
            product_root: std::path::PathBuf::from("/private/tmp/t"),
            workspace_path: std::path::PathBuf::from("/private/tmp/t/workspaces/default"),
            schema_version: 1
        }
    }
}

/// Takes the lease and starts a server on it.
fn start(root: &ProductRoot) -> ServerHandle
{
    let lease = Arc::new(
        root.acquire_writer_lease()
            .expect("the lease should be free")
    );

    SocketServer::bind(lease, Arc::new(Fixed))
        .expect("the socket should bind")
        .spawn()
}

#[test]
fn the_cli_client_gets_a_typed_answer_over_a_real_socket()
{
    let root = scratch_root("sock-round-trip");
    let server = start(&root);

    let mut client = AppClient::connect(&root).expect("the client should connect");
    let status = client.app_status().expect("the app should answer");

    assert_eq!(status.pid, std::process::id());
    assert!(status.holds_writer_lease);
    assert_eq!(status.schema_version, 1);

    server.stop();
}

#[test]
fn the_socket_is_reachable_only_by_its_owner()
{
    let root = scratch_root("sock-modes");
    let server = start(&root);

    assert_eq!(mode_of(server.path()).unwrap(), PRIVATE_FILE_MODE);
    assert_eq!(mode_of(&root.run_dir()).unwrap(), 0o700);

    server.stop();
}

// The pitfall the brief names, over a real connection: a frame that arrives in
// pieces is still one frame.
#[test]
fn a_request_split_across_writes_is_answered_once()
{
    let root = scratch_root("sock-split");
    let server = start(&root);

    let request = SocketRequest::new(fresh_id(), SocketMethod::AppStatus);
    let line = request.to_json_line().unwrap();
    let mut stream = connect(&root);

    for byte in line.as_bytes()
    {
        stream
            .write_all(&[*byte])
            .expect("a byte could not be sent");
        std::thread::sleep(Duration::from_micros(200));
    }

    let answers = read_answers(stream, 1);

    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].id, request.id);

    server.stop();
}

// The other half: several frames in one write are several answers, in order.
#[test]
fn several_requests_in_one_write_are_answered_in_order()
{
    let root = scratch_root("sock-coalesced");
    let server = start(&root);

    let requests: Vec<SocketRequest> = (0..3)
        .map(|_| SocketRequest::new(fresh_id(), SocketMethod::AppStatus))
        .collect();
    let batch: String = requests
        .iter()
        .map(|request| request.to_json_line().unwrap())
        .collect();

    let mut stream = connect(&root);
    stream
        .write_all(batch.as_bytes())
        .expect("the batch could not be sent");

    let answers = read_answers(stream, requests.len());

    assert_eq!(
        answers.iter().map(|answer| answer.id).collect::<Vec<_>>(),
        requests
            .iter()
            .map(|request| request.id)
            .collect::<Vec<_>>()
    );

    server.stop();
}

// A caller that connects and says nothing must not hold the server open for
// ever. The bound is `REQUEST_READ_TIMEOUT`, and what the caller sees is the
// connection closing rather than an answer.
#[test]
fn a_client_that_says_nothing_is_let_go_and_does_not_wedge_the_server()
{
    let root = scratch_root("sock-silent");
    let server = start(&root);

    let mut silent = UnixStream::connect(root.socket_path()).expect("the client should connect");
    silent
        .set_read_timeout(Some(REQUEST_READ_TIMEOUT * 4))
        .unwrap();

    // While it sits there, the server still answers everyone else.
    let mut client = AppClient::connect(&root).expect("the client should connect");
    client.app_status().expect("the app should still answer");

    // And the silent one is let go rather than held for ever. Reading returns
    // zero bytes once the server has closed its end.
    let mut nothing = [0u8; 1];

    assert_eq!(
        silent.read(&mut nothing).ok(),
        Some(0),
        "a client that said nothing was never let go"
    );

    server.stop();
}

#[test]
fn a_method_this_build_does_not_serve_is_refused_over_the_socket()
{
    let root = scratch_root("sock-unavailable");
    let server = start(&root);

    let request = SocketRequest::new(fresh_id(), SocketMethod::UiFocus);
    let mut stream = connect(&root);
    stream
        .write_all(request.to_json_line().unwrap().as_bytes())
        .expect("the request could not be sent");

    let answers = read_answers(stream, 1);

    match &answers[0].outcome
    {
        SocketOutcome::Error(envelope) =>
        {
            assert_eq!(envelope.code, ErrorCode::AppProtocolVersion)
        }
        SocketOutcome::Ok(_) => panic!("a method this build has not built answered")
    }

    server.stop();
}

// The stale socket case end to end, and the reason the lease decides it: after
// an app dies its socket file is still on disk, and the next start has to bind
// over it without ever asking the file whether anyone is home.
#[test]
fn a_socket_file_left_by_a_dead_app_does_not_block_the_next_start()
{
    let root = scratch_root("sock-stale");
    let lease = Arc::new(root.acquire_writer_lease().unwrap());
    let server = SocketServer::bind(Arc::clone(&lease), Arc::new(Fixed)).unwrap();
    let path = server.path().to_path_buf();

    // Drop the listener without the handle that would tidy up, which is what a
    // process that died leaves behind.
    drop(server);
    assert!(
        path.exists(),
        "this test needs the socket file to be left behind"
    );

    let server = SocketServer::bind(lease, Arc::new(Fixed))
        .expect("a socket file left by a dead app blocked the next start")
        .spawn();

    let mut client = AppClient::connect(&root).expect("the client should connect");
    client.app_status().expect("the new server should answer");

    server.stop();
}

#[test]
fn a_second_app_cannot_bind_while_the_first_holds_the_lease()
{
    let root = scratch_root("sock-single");
    let server = start(&root);

    assert!(
        root.acquire_writer_lease().is_err(),
        "two apps held the writer lease at once"
    );

    server.stop();
}

#[test]
fn a_client_with_nothing_to_talk_to_says_the_app_is_not_running()
{
    let root = scratch_root("sock-absent");

    assert!(matches!(
        AppClient::connect(&root),
        Err(ClientError::NotRunning)
    ));

    // A socket file with nothing listening on it is the same answer, not a
    // different one: the file is not what decides.
    root.prepare().unwrap();
    std::fs::write(root.socket_path(), b"").unwrap();

    assert!(matches!(
        AppClient::connect(&root),
        Err(ClientError::NotRunning)
    ));
}

// The `sun_path` limit, from the client side. The server side of the same check
// is a unit test, because a root this long cannot be made to bind.
#[test]
fn a_socket_path_longer_than_a_sockaddr_is_refused_as_itself()
{
    let root = ProductRoot::at(format!("/private/tmp/{}", "x".repeat(120)));

    assert!(matches!(
        AppClient::connect(&root),
        Err(ClientError::PathTooLong { .. })
    ));
}

/// Connects straight to the socket, for the tests that speak the protocol by
/// hand rather than through [`AppClient`].
fn connect(root: &ProductRoot) -> UnixStream
{
    let stream = UnixStream::connect(root.socket_path()).expect("the client could not connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    stream
}

/// Reads until the expected number of answers has arrived, then to the end.
fn read_answers(stream: UnixStream, expected: usize) -> Vec<SocketResponse<AppStatusResult>>
{
    let mut text = String::new();
    let mut reader = stream;

    while text.matches('\n').count() < expected
    {
        let mut chunk = [0u8; 1024];

        match reader.read(&mut chunk)
        {
            Ok(0) => break,
            Ok(read) => text.push_str(&String::from_utf8_lossy(&chunk[..read])),
            Err(error) => panic!("no answer arrived: {error}")
        }
    }

    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            SocketResponse::from_json_line(line)
                .unwrap_or_else(|error| panic!("the answer was not readable: {error}"))
        })
        .collect()
}

/// A request id, minted the way the runtime mints one.
fn fresh_id() -> trdr_core::RequestId
{
    trdr_core::RequestId::from_ulid(ulid::Ulid::generate())
}
