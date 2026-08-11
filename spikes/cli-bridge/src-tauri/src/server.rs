use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use serde_json::json;

use crate::approval::{ApprovalSummary, Broker, Hashes, Outcome, Started};
use crate::peer;
use crate::protocol::{self, ClickParams, Method, RegisterParams, Request, Response, SetHashesParams, PROTOCOL_VERSION};
use crate::runtime::{self, WriterLease};

/// What the socket server needs from the app around it.
///
/// It exists so the protocol, the dedupe and the disconnect handling can be
/// exercised without a running NSApplication. Everything below this line is
/// reachable from `cargo test`; the AppKit half is behind this trait.
pub trait Host: Send + Sync + 'static
{
    fn open_ui(&self, resource: &str, id: &str) -> Result<(), String>;
    fn present_approval(&self, summary: ApprovalSummary);
    fn dismiss_approval(&self, request_id: &str);
    /// Spike-only. Presses a button on the sheet that is on screen.
    fn click_sheet(&self, approve: bool) -> Result<(), String>;
    fn log(&self, line: String);
}

pub struct Bridge
{
    pub broker: Arc<Broker>,
    pub host: Arc<dyn Host>
}

/// Binds the socket, holding the writer lease for as long as the returned value
/// lives.
///
/// The order matters. The lease is taken first, and only its holder is entitled
/// to remove a socket file: whoever holds it is the only live app, so a socket
/// file it finds was left behind by a dead one.
pub fn bind(root_run_dir: &Path) -> std::io::Result<(WriterLease, UnixListener)>
{
    let lease = runtime::acquire_lease(&root_run_dir.join("writer.lock"))?;
    let socket = root_run_dir.join("app.sock");
    runtime::clear_stale_socket(&lease, &socket)?;

    let listener = UnixListener::bind(&socket)?;
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))?;
    Ok((lease, listener))
}

impl Bridge
{
    pub fn serve(self: Arc<Self>, listener: UnixListener)
    {
        for connection in listener.incoming()
        {
            match connection
            {
                Ok(stream) =>
                {
                    let bridge = Arc::clone(&self);
                    thread::spawn(move || bridge.converse(stream));
                }
                Err(error) => self.host.log(format!("accept failed: {error}"))
            }
        }
    }

    fn converse(&self, stream: UnixStream)
    {
        let mut out = match stream.try_clone()
        {
            Ok(clone) => clone,
            Err(error) => return self.host.log(format!("connection could not be split: {error}"))
        };

        if !peer::is_owner(&stream).unwrap_or(false)
        {
            self.host.log("a connection from another user was refused".to_string());
            let _ = out.write_all(Response::failed(None, "PEER_REJECTED", "this socket serves one user").line().as_bytes());
            return;
        }

        for line in BufReader::new(&stream).lines()
        {
            let Ok(line) = line else { return };
            if line.trim().is_empty()
            {
                continue;
            }
            let response = self.answer(&line, &stream);
            self.host.log(format!("{line} → {}", response.code().unwrap_or("ok")));
            if out.write_all(response.line().as_bytes()).is_err()
            {
                return;
            }
        }
    }

    fn answer(&self, line: &str, stream: &UnixStream) -> Response
    {
        let request = match protocol::read_request(line)
        {
            Ok(request) => request,
            Err(refusal) => return refusal
        };
        let Some(method) = Method::parse(&request.method) else
        {
            return Response::failed(Some(request.id), "UNKNOWN_METHOD", format!("`{}` is not a method", request.method));
        };

        match method
        {
            Method::AppStatus => Response::ok(&request.id, json!({
                "running": true,
                "pid": std::process::id(),
                "protocol": PROTOCOL_VERSION
            })),
            Method::UiOpen => self.open_ui(&request),
            Method::SpikeSetHashes => self.set_hashes(&request),
            Method::SpikeClick => self.click_sheet(&request),
            Method::StrategyRegisterRequest => self.register(&request, stream)
        }
    }

    fn open_ui(&self, request: &Request) -> Response
    {
        let resource = request.params.get("resource").and_then(|value| value.as_str()).unwrap_or_default();
        let id = request.params.get("id").and_then(|value| value.as_str()).unwrap_or_default();
        match self.host.open_ui(resource, id)
        {
            Ok(()) => Response::ok(&request.id, json!({"status": "opened"})),
            Err(problem) => Response::failed(Some(request.id.clone()), "UI_UNAVAILABLE", problem)
        }
    }

    fn set_hashes(&self, request: &Request) -> Response
    {
        let params: SetHashesParams = match serde_json::from_value(request.params.clone())
        {
            Ok(params) => params,
            Err(error) => return Response::failed(Some(request.id.clone()), "BAD_REQUEST", error.to_string())
        };
        self.broker.set_hashes(&params.strategy_id, Hashes { spec: params.spec_hash, data: params.data_hash });
        Response::ok(&request.id, json!({"status": "moved"}))
    }

    fn click_sheet(&self, request: &Request) -> Response
    {
        let params: ClickParams = match serde_json::from_value(request.params.clone())
        {
            Ok(params) => params,
            Err(error) => return Response::failed(Some(request.id.clone()), "BAD_REQUEST", error.to_string())
        };
        match self.host.click_sheet(params.approve)
        {
            Ok(()) => Response::ok(&request.id, json!({"status": "clicked"})),
            Err(problem) => Response::failed(Some(request.id.clone()), "NO_SHEET", problem)
        }
    }

    /// The round trip of section 9.3, from this side of the socket.
    fn register(&self, request: &Request, stream: &UnixStream) -> Response
    {
        let params: RegisterParams = match serde_json::from_value(request.params.clone())
        {
            Ok(params) => params,
            Err(error) => return Response::failed(Some(request.id.clone()), "BAD_REQUEST", error.to_string())
        };

        let frozen = Hashes { spec: params.spec_hash.clone(), data: params.data_hash.clone() };
        let waiting = match self.broker.start(&request.id, &params.strategy_id, frozen.clone())
        {
            Started::Waiting(receiver) => receiver,
            Started::AlreadySettled(outcome) => return terminal(&request.id, outcome),
            Started::Busy => return Response::failed(
                Some(request.id.clone()),
                "APPROVAL_BUSY",
                "another approval is on screen; one is shown at a time"
            )
        };

        let summary = ApprovalSummary::build(&request.id, &params.strategy_id, &params.command, &frozen, params.expires_in_seconds);
        self.host.present_approval(summary);
        self.start_expiry(&request.id, params.expires_in_seconds);
        self.watch_for_disconnect(&request.id, stream);

        // The wait itself. The caller's socket stays open across it, which is
        // what "the same socket request receives terminal result" means.
        let outcome = waiting.recv().unwrap_or(Outcome::Cancelled);
        self.host.dismiss_approval(&request.id);
        terminal(&request.id, outcome)
    }

    fn start_expiry(&self, request_id: &str, seconds: u64)
    {
        let broker = Arc::clone(&self.broker);
        let request_id = request_id.to_string();
        thread::spawn(move ||
        {
            thread::sleep(crate::approval::expiry_duration(seconds));
            broker.settle(&request_id, Outcome::Expired);
        });
    }

    /// Settles the approval as cancelled when the caller goes away.
    ///
    /// It polls rather than reads, because the connection thread owns the read
    /// side and a second reader would steal a line the caller pipelined. `poll`
    /// followed by a peek leaves the byte stream untouched, and the loop lets the
    /// watcher stand down as soon as the request stops being the live one — a
    /// blocking read could not be called off.
    fn watch_for_disconnect(&self, request_id: &str, stream: &UnixStream)
    {
        let Ok(watched) = stream.try_clone() else { return };
        let broker = Arc::clone(&self.broker);
        let request_id = request_id.to_string();
        thread::spawn(move ||
        {
            while broker.live_request_id().as_deref() == Some(request_id.as_str())
            {
                if peer_has_gone(&watched)
                {
                    broker.settle(&request_id, Outcome::Cancelled);
                    return;
                }
            }
        });
    }
}

const POLL_INTERVAL: Duration = Duration::from_millis(100);

fn peer_has_gone(stream: &UnixStream) -> bool
{
    let mut watched = libc::pollfd { fd: stream.as_raw_fd(), events: libc::POLLIN, revents: 0 };
    let ready = unsafe { libc::poll(&mut watched, 1, POLL_INTERVAL.as_millis() as i32) };
    if ready <= 0
    {
        return false;
    }
    if watched.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0
    {
        return true;
    }
    // Readable can mean either a byte or end of stream. Only a peek tells them
    // apart, and a peek does not take the byte out of the caller's line.
    let mut byte = 0u8;
    let peeked = unsafe { libc::recv(stream.as_raw_fd(), std::ptr::addr_of_mut!(byte).cast(), 1, libc::MSG_PEEK) };
    peeked == 0
}

fn terminal(request_id: &str, outcome: Outcome) -> Response
{
    // Approved is the only outcome that wrote anything, so it is the only one the
    // caller should read as success. A rejected or expired request is a refusal,
    // and a CLI that exits 0 on it teaches its users to stop reading.
    match outcome
    {
        Outcome::Approved => Response::ok(request_id, json!({"status": "registered"})),
        other => Response::failed(Some(request_id.to_string()), other.code(), match other
        {
            Outcome::Rejected => "the user rejected this registration",
            Outcome::Expired => "the approval expired before it was answered",
            Outcome::Cancelled => "the request was cancelled",
            Outcome::Stale => "the spec or data hash moved while the approval was on screen",
            Outcome::Approved => unreachable!()
        })
    }
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::sync::Mutex;

    /// A host that answers approvals the way a test needs, without AppKit.
    struct FakeHost
    {
        broker: Mutex<Option<Arc<Broker>>>,
        answer: Option<Outcome>,
        presented: Mutex<Vec<ApprovalSummary>>,
        dismissed: Mutex<Vec<String>>,
        ui_opens: Mutex<Vec<(String, String)>>
    }

    impl FakeHost
    {
        fn new(answer: Option<Outcome>) -> Arc<FakeHost>
        {
            Arc::new(FakeHost
            {
                broker: Mutex::new(None),
                answer,
                presented: Mutex::new(Vec::new()),
                dismissed: Mutex::new(Vec::new()),
                ui_opens: Mutex::new(Vec::new())
            })
        }
    }

    impl Host for FakeHost
    {
        fn open_ui(&self, resource: &str, id: &str) -> Result<(), String>
        {
            self.ui_opens.lock().unwrap().push((resource.to_string(), id.to_string()));
            Ok(())
        }

        fn present_approval(&self, summary: ApprovalSummary)
        {
            self.presented.lock().unwrap().push(summary.clone());
            let Some(answer) = self.answer else { return };
            let broker = self.broker.lock().unwrap().clone().expect("the fake host was not given a broker");
            thread::spawn(move ||
            {
                thread::sleep(Duration::from_millis(20));
                broker.settle(&summary.request_id, answer);
            });
        }

        fn dismiss_approval(&self, request_id: &str)
        {
            self.dismissed.lock().unwrap().push(request_id.to_string());
        }

        fn click_sheet(&self, _approve: bool) -> Result<(), String>
        {
            Err("there is no sheet without a running NSApplication".to_string())
        }

        fn log(&self, _line: String) {}
    }

    struct Harness
    {
        socket: std::path::PathBuf,
        host: Arc<FakeHost>,
        broker: Arc<Broker>,
        _lease: WriterLease
    }

    fn harness(name: &str, answer: Option<Outcome>) -> Harness
    {
        let root = std::env::temp_dir().join(format!("trdr-spike-bridge-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        let run = runtime::prepare_run_dir(&root).expect("the run directory could not be made");

        let (lease, listener) = bind(&run).expect("the socket could not be bound");
        let broker = Arc::new(Broker::default());
        let host = FakeHost::new(answer);
        *host.broker.lock().unwrap() = Some(Arc::clone(&broker));

        let bridge = Arc::new(Bridge { broker: Arc::clone(&broker), host: Arc::clone(&host) as Arc<dyn Host> });
        thread::spawn(move || bridge.serve(listener));

        Harness { socket: run.join("app.sock"), host, broker, _lease: lease }
    }

    impl Harness
    {
        fn connect(&self) -> UnixStream
        {
            UnixStream::connect(&self.socket).expect("the client could not connect")
        }

        fn ask(&self, line: &str) -> Response
        {
            let stream = self.connect();
            send(&stream, line);
            read_one(&stream)
        }
    }

    fn send(stream: &UnixStream, line: &str)
    {
        let mut writer = stream.try_clone().expect("the stream could not be cloned");
        writer.write_all(format!("{line}\n").as_bytes()).expect("the request could not be sent");
    }

    fn read_one(stream: &UnixStream) -> Response
    {
        let mut text = String::new();
        BufReader::new(stream).read_line(&mut text).expect("no response arrived");
        serde_json::from_str(&text).unwrap_or_else(|error| panic!("the response was not readable: {error}\n{text}"))
    }

    fn register_line(id: &str, seconds: u64) -> String
    {
        json!({
            "v": 1,
            "id": id,
            "method": "strategy.register.request",
            "params": {
                "strategy_id": "low-vol-v1",
                "command": "trdr strategy register low-vol-v1",
                "spec_hash": "spec-1",
                "data_hash": "data-1",
                "expires_in_seconds": seconds
            }
        })
        .to_string()
    }

    #[test]
    fn app_status_answers_over_the_socket()
    {
        let harness = harness("status", None);
        let response = harness.ask(r#"{"v":1,"id":"r1","method":"app.status"}"#);
        assert!(response.ok);
        assert_eq!(response.result.expect("a result was expected")["protocol"], PROTOCOL_VERSION);
    }

    #[test]
    fn an_unknown_method_is_refused_by_name()
    {
        let harness = harness("unknown-method", None);
        let response = harness.ask(r#"{"v":1,"id":"r1","method":"registration.commit"}"#);
        assert_eq!(response.code(), Some("UNKNOWN_METHOD"));
        assert_eq!(response.id.as_deref(), Some("r1"));
    }

    #[test]
    fn ui_open_reaches_the_host()
    {
        let harness = harness("ui-open", None);
        let response = harness.ask(r#"{"v":1,"id":"r1","method":"ui.open","params":{"resource":"strategy","id":"low-vol-v1"}}"#);
        assert!(response.ok);
        assert_eq!(harness.host.ui_opens.lock().unwrap().as_slice(), [("strategy".to_string(), "low-vol-v1".to_string())]);
    }

    #[test]
    fn an_approval_returns_to_the_same_waiting_connection()
    {
        let harness = harness("approve", Some(Outcome::Approved));
        let response = harness.ask(&register_line("r1", 30));
        assert!(response.ok, "an approved registration should succeed: {response:?}");
        assert_eq!(response.id.as_deref(), Some("r1"));
        assert_eq!(harness.broker.committed(), vec!["low-vol-v1".to_string()]);
    }

    #[test]
    fn a_rejection_returns_to_the_same_waiting_connection_and_writes_nothing()
    {
        let harness = harness("reject", Some(Outcome::Rejected));
        let response = harness.ask(&register_line("r1", 30));
        assert_eq!(response.code(), Some("REJECTED"));
        assert!(harness.broker.committed().is_empty());
    }

    #[test]
    fn an_expiry_returns_to_the_same_waiting_connection()
    {
        let harness = harness("expire", None);
        let response = harness.ask(&register_line("r1", 1));
        assert_eq!(response.code(), Some("EXPIRED"));
        assert!(harness.broker.committed().is_empty());
    }

    // The pitfall the brief names: the same request id must not raise a second
    // sheet, and must not write twice.
    #[test]
    fn a_repeated_request_id_replays_its_result_without_asking_again()
    {
        let harness = harness("dedupe", Some(Outcome::Approved));
        assert!(harness.ask(&register_line("r1", 30)).ok);
        assert!(harness.ask(&register_line("r1", 30)).ok);

        assert_eq!(harness.broker.committed(), vec!["low-vol-v1".to_string()], "the registration was written twice");
        assert_eq!(harness.host.presented.lock().unwrap().len(), 1, "a second sheet was raised for a settled request");
    }

    // The other pitfall: a caller that goes away mid-approval cancels its own
    // request rather than leaving a sheet nobody is waiting on.
    #[test]
    fn a_caller_that_disconnects_cancels_its_own_approval()
    {
        let harness = harness("disconnect", None);
        let stream = harness.connect();
        send(&stream, &register_line("r1", 30));

        while harness.broker.live_request_id().is_none()
        {
            thread::sleep(Duration::from_millis(5));
        }
        drop(stream);

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while harness.broker.live_request_id().is_some() && std::time::Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(harness.broker.live_request_id().is_none(), "the approval outlived the caller that asked for it");
        assert!(harness.broker.committed().is_empty());
    }

    #[test]
    fn a_second_approval_is_refused_while_one_is_on_screen()
    {
        let harness = harness("busy", None);
        let first = harness.connect();
        send(&first, &register_line("r1", 30));
        while harness.broker.live_request_id().is_none()
        {
            thread::sleep(Duration::from_millis(5));
        }

        let response = harness.ask(&register_line("r2", 30));
        assert_eq!(response.code(), Some("APPROVAL_BUSY"));
    }

    #[test]
    fn an_approval_whose_hashes_moved_is_refused_as_stale()
    {
        let harness = harness("stale", None);
        let stream = harness.connect();
        send(&stream, &register_line("r1", 30));
        while harness.broker.live_request_id().is_none()
        {
            thread::sleep(Duration::from_millis(5));
        }

        harness.ask(r#"{"v":1,"id":"m1","method":"spike.set_hashes","params":{"strategy_id":"low-vol-v1","spec_hash":"spec-2","data_hash":"data-1"}}"#);
        harness.broker.settle("r1", Outcome::Approved);

        let response = read_one(&stream);
        assert_eq!(response.code(), Some("STALE_APPROVAL"));
        assert!(harness.broker.committed().is_empty());
    }

    #[test]
    fn the_socket_file_is_reachable_only_by_its_owner()
    {
        let harness = harness("socket-mode", None);
        assert_eq!(runtime::mode_of(&harness.socket).expect("the socket should exist"), 0o600);
    }

    // The stale-socket case, end to end: a socket file left behind by an app that
    // died does not stop the next one from binding.
    #[test]
    fn a_socket_file_left_by_a_dead_app_does_not_block_the_next_start()
    {
        let root = std::env::temp_dir().join("trdr-spike-bridge-restart");
        let _ = std::fs::remove_dir_all(&root);
        let run = runtime::prepare_run_dir(&root).expect("the run directory could not be made");

        let (lease, listener) = bind(&run).expect("the first bind should work");
        drop(listener);
        drop(lease);
        assert!(run.join("app.sock").exists(), "this test needs the socket file to be left behind");

        bind(&run).expect("a socket file left by a dead app blocked the next start");
    }

    #[test]
    fn a_second_app_cannot_bind_while_the_first_holds_the_lease()
    {
        let root = std::env::temp_dir().join("trdr-spike-bridge-single");
        let _ = std::fs::remove_dir_all(&root);
        let run = runtime::prepare_run_dir(&root).expect("the run directory could not be made");

        let _first = bind(&run).expect("the first bind should work");
        assert!(bind(&run).is_err(), "two apps bound the same socket at once");
    }
}
