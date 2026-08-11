//! The Unix socket the `trdr` CLI talks to, and the client half that talks to it.
//!
//! Section 9.2 of `docs/FOUNDATION_DESIGN.md` fixes the protocol: one JSON object
//! per line, a version and a request id on every frame, and an unknown method,
//! field, or version refused rather than ignored. All of that lives in
//! [`trdr_core::socket`]; this module is the socket, the framing, and who is
//! allowed to speak.
//!
//! # Who may connect
//!
//! Two checks, not one. The socket file is `0600` inside a `0700` directory, and
//! every accepted connection is asked for its peer's uid through `getpeereid`.
//! The first is a property of a path and stops being true if a descriptor
//! outlives a mode change or the socket ever moves; the second is recorded by the
//! kernel when the peer connects and cannot be restated by the caller.
//!
//! # Which socket file is stale
//!
//! The writer lease decides, and nothing else does. Whoever holds the lease is
//! the only live app, so a socket file it finds at bind time was left behind by
//! one that died, and unlinking it is safe. The tempting alternative — connect to
//! the socket and see whether anything answers — is worse in both directions: an
//! app that is alive but wedged answers nothing and would be declared dead, and a
//! path whose inode was replaced answers something else entirely. [`bind`] takes
//! a [`WriterLease`] for exactly this reason: the unlink is not reachable without
//! one.

use crate::root::{ProductRoot, WriterLease, PRIVATE_FILE_MODE};
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use trdr_core::error::{ErrorCode, ErrorEnvelope, ErrorParam, Retryability};
use trdr_core::socket::{
    AccountInspectResult, AppStatusResult, DataCoverageResult, FrameError, SocketMethod,
    SocketOutcome, SocketRequest, SocketResponse, MAX_FRAME_BYTES
};
use trdr_core::RequestId;
use ulid::Ulid;

/// The most a `sockaddr_un` path may be on macOS, without its terminating zero.
///
/// `sun_path` is 104 bytes. A path over the limit fails at `bind` with an error
/// about the address rather than about the length, which is why it is checked
/// here and reported as itself.
pub const MAX_SOCKET_PATH_BYTES: usize = 103;

/// How long a connected client may leave the server waiting for its frame.
///
/// A caller that connects and then says nothing costs one thread until this
/// elapses, and then the connection is dropped. The bound is deliberately short:
/// a request the CLI already decided to send is one write away, and section 9.3's
/// long wait belongs to the approval round trip, which will hold its own
/// connection open on purpose.
pub const REQUEST_READ_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the server will wait for a client to take its answer.
pub const RESPONSE_WRITE_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the CLI waits for an answer before giving up on a wedged app.
pub const CLIENT_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the accept loop looks at the stop flag.
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// What the socket server needs from the app around it.
///
/// The test seam section 13 calls `AppBridge`. Everything below it is reachable
/// from `cargo test` without a window, a WebView, or an NSApplication, which is
/// what lets the protocol be proven before the app exists.
pub trait AppBridge: Send + Sync + 'static
{
    /// Answers `app.status` (section 9.2).
    fn app_status(&self) -> AppStatusResult;

    /// Answers `account.inspect` with the same values the Today screen shows.
    ///
    /// Fallible where `app.status` is not, because this one reads a model that
    /// can fail to build, and section 9.2 gives a read an error arm for exactly
    /// that. `app.status` has none because a process that could not answer it
    /// would not be accepting on the socket in the first place.
    fn account_inspect(&self) -> Result<AccountInspectResult, ErrorEnvelope>;

    /// Answers `data.coverage` with the same coverage the Lab draft shows.
    fn data_coverage(&self) -> Result<DataCoverageResult, ErrorEnvelope>;
}

/// The Unix socket server of section 9.2, bound and not yet accepting.
pub struct SocketServer
{
    listener: UnixListener,
    served: Arc<Served>,
    path: PathBuf,
    lease: Arc<WriterLease>
}

/// What each connection thread needs, shared between them.
struct Served
{
    bridge: Arc<dyn AppBridge>,
    /// The only uid this socket answers. Read once at bind, so a later change to
    /// the process's identity cannot widen it.
    owner_uid: libc::uid_t
}

impl SocketServer
{
    /// Binds `<product root>/run/app.sock`, clearing a socket file a dead app
    /// left behind.
    ///
    /// The lease is the argument because clearing that file is only safe for its
    /// holder, and because the app that answers questions about the database has
    /// to be the one holding it.
    pub fn bind(lease: Arc<WriterLease>, bridge: Arc<dyn AppBridge>) -> Result<Self, ServerError>
    {
        let root = lease.root().clone();
        root.prepare().map_err(|source| ServerError::Prepare {
            source: Box::new(source)
        })?;

        let path = root.socket_path();
        check_socket_path(&path)?;

        // Only reachable with a lease in hand: see the module documentation.
        clear_stale_socket(&lease, &path)?;

        let listener = UnixListener::bind(&path).map_err(|source| ServerError::Bind {
            path: path.clone(),
            source
        })?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE))
            .map_err(|source| ServerError::Bind {
                path: path.clone(),
                source
            })?;
        listener
            .set_nonblocking(true)
            .map_err(|source| ServerError::Bind {
                path: path.clone(),
                source
            })?;

        // SAFETY: `getuid` reads this process's own identity and cannot fail.
        let owner_uid = unsafe { libc::getuid() };

        Ok(Self {
            listener,
            served: Arc::new(Served { bridge, owner_uid }),
            path,
            lease
        })
    }

    /// Where the socket is.
    pub fn path(&self) -> &Path
    {
        &self.path
    }

    /// Starts answering, on a thread of its own.
    ///
    /// Dropping the returned handle stops the loop, waits for it, and removes the
    /// socket file — which is safe to do because the handle still holds the lease.
    pub fn spawn(self) -> ServerHandle
    {
        let stop = Arc::new(AtomicBool::new(false));
        let path = self.path.clone();
        let lease = Arc::clone(&self.lease);
        let loop_stop = Arc::clone(&stop);
        let thread = thread::spawn(move || self.accept_until_stopped(&loop_stop));

        ServerHandle {
            stop,
            thread: Some(thread),
            path,
            _lease: lease
        }
    }

    /// Accepts connections until the flag is set.
    fn accept_until_stopped(self, stop: &AtomicBool)
    {
        while !stop.load(Ordering::Relaxed)
        {
            match self.listener.accept()
            {
                Ok((stream, _)) =>
                {
                    let served = Arc::clone(&self.served);
                    thread::spawn(move || served.converse(stream));
                }
                // Nothing waiting. Sleeping on the descriptor rather than on the
                // clock keeps the loop from spinning without making the stop flag
                // wait for a connection that may never come.
                Err(error) if error.kind() == io::ErrorKind::WouldBlock =>
                {
                    wait_readable(self.listener.as_raw_fd(), ACCEPT_POLL_INTERVAL);
                }
                // An accept that failed for another reason says nothing about the
                // next one. Pausing first stops a permanent failure from turning
                // into a busy loop.
                Err(_) => thread::sleep(ACCEPT_POLL_INTERVAL)
            }
        }
    }
}

impl Served
{
    /// Reads frames from one connection and answers them until it ends.
    fn converse(&self, stream: UnixStream)
    {
        // POSIX does not pass O_NONBLOCK to an accepted socket, but saying so
        // costs nothing and makes the timeouts below mean what they say.
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_read_timeout(Some(REQUEST_READ_TIMEOUT));
        let _ = stream.set_write_timeout(Some(RESPONSE_WRITE_TIMEOUT));

        // Before a byte is read. A connection from another user is closed
        // without an answer: it is told nothing, because there is nothing it is
        // entitled to know.
        match peer_uid(&stream)
        {
            Ok(uid) if uid == self.owner_uid =>
            {}
            _ => return
        }

        let Ok(mut out) = stream.try_clone()
        else
        {
            return;
        };
        let mut frames = FrameReader::new(stream);

        loop
        {
            let response = match frames.next_frame()
            {
                Ok(Some(line)) => self.answer(&line),
                // The peer finished speaking, or stopped without finishing a
                // frame, or sent more than a frame may weigh. None of those leave
                // anything worth saying on a stream that is already over.
                Ok(None) | Err(_) => return
            };

            let Ok(line) = response.to_json_line()
            else
            {
                return;
            };

            if out.write_all(line.as_bytes()).is_err()
            {
                return;
            }
        }
    }

    /// Turns one line into the line that answers it.
    fn answer(&self, line: &str) -> SocketResponse<serde_json::Value>
    {
        let request = match SocketRequest::from_json_line(line)
        {
            Ok(request) => request,
            // A frame that could not be read has no usable request id, so the
            // refusal carries none. `FrameError` already knows how to say which
            // rule was broken without echoing what was sent.
            Err(refusal) => return refused(refusal)
        };

        match &request.method
        {
            SocketMethod::AppStatus => match serde_json::to_value(self.bridge.app_status())
            {
                Ok(result) => SocketResponse::ok(request.id, result),
                Err(_) => SocketResponse::error(
                    request.id,
                    ErrorEnvelope::new(ErrorCode::AppProtocolVersion)
                        .with_request(request.id)
                        .with_param("reason", ErrorParam::literal("result_unserialisable"))
                )
            },
            SocketMethod::AccountInspect => answer_read(request.id, self.bridge.account_inspect()),
            SocketMethod::DataCoverage => answer_read(request.id, self.bridge.data_coverage()),
            // Every other method by name, so that adding one to the protocol is a
            // compile error here rather than a method that silently does nothing.
            method @ (SocketMethod::UiOpen(_)
            | SocketMethod::UiFocus
            | SocketMethod::IngestRequest(_)
            | SocketMethod::BacktestRun
            | SocketMethod::BacktestInspect
            | SocketMethod::BackupCreateRequest(_)
            | SocketMethod::BackupVerify(_)
            | SocketMethod::WorkspaceRestoreRequest(_)
            | SocketMethod::StrategyInspect(_)
            | SocketMethod::StrategyRegisterRequest(_)) => SocketResponse::error(
                request.id,
                ErrorEnvelope::new(ErrorCode::AppProtocolVersion)
                    .with_request(request.id)
                    .with_param("reason", ErrorParam::literal("method_unavailable"))
                    .with_param("method", ErrorParam::literal(method.name()))
            )
        }
    }
}

/// Turns a read's result into the response frame that goes back.
///
/// Split out because the two reads below would otherwise repeat the same six
/// lines, and because the serialisation failure has one right answer: a result
/// that cannot be written is a protocol failure, not a data one, and the caller
/// needs to be told that rather than left waiting.
fn answer_read<R: serde::Serialize>(
    id: RequestId,
    result: Result<R, ErrorEnvelope>
) -> SocketResponse<serde_json::Value>
{
    let value = match result
    {
        Ok(value) => value,
        Err(error) => return SocketResponse::error(id, error.with_request(id))
    };

    match serde_json::to_value(value)
    {
        Ok(result) => SocketResponse::ok(id, result),
        Err(_) => SocketResponse::error(
            id,
            ErrorEnvelope::new(ErrorCode::AppProtocolVersion)
                .with_request(id)
                .with_param("reason", ErrorParam::literal("result_unserialisable"))
        )
    }
}

/// A response to a frame that could not be read far enough to have an id.
///
/// The id on the wire is not optional, so a refusal that has none borrows a fresh
/// one rather than inventing a shape the protocol does not have. The caller can
/// still tell the frame was refused, and by which rule, from the envelope.
fn refused(refusal: FrameError) -> SocketResponse<serde_json::Value>
{
    let id = fresh_request_id();
    SocketResponse::error(id, refusal.to_envelope(Some(id)))
}

/// The server, running, with the socket file and the lease still held.
pub struct ServerHandle
{
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    path: PathBuf,
    /// Kept so the lease cannot be released while the socket is still bound.
    _lease: Arc<WriterLease>
}

impl ServerHandle
{
    /// Where the socket is.
    pub fn path(&self) -> &Path
    {
        &self.path
    }

    /// Stops accepting and waits for the loop to finish.
    pub fn stop(self)
    {
        drop(self);
    }
}

impl Drop for ServerHandle
{
    fn drop(&mut self)
    {
        self.stop.store(true, Ordering::Relaxed);

        if let Some(thread) = self.thread.take()
        {
            let _ = thread.join();
        }

        // Safe for the same reason the unlink at bind time is: this handle still
        // holds the lease, so nothing else is serving on this path.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// The client half: what `trdr` uses to ask a running app something.
pub struct AppClient
{
    out: UnixStream,
    frames: FrameReader<UnixStream>
}

impl AppClient
{
    /// Connects to the app serving this product root.
    ///
    /// A socket that is not there, or that nothing is listening on, is the
    /// ordinary "no app is running" case rather than a failure to report.
    pub fn connect(root: &ProductRoot) -> Result<Self, ClientError>
    {
        let path = root.socket_path();

        if path.as_os_str().len() > MAX_SOCKET_PATH_BYTES
        {
            return Err(ClientError::PathTooLong {
                found: path.as_os_str().len(),
                limit: MAX_SOCKET_PATH_BYTES
            });
        }

        let stream = UnixStream::connect(&path).map_err(|_| ClientError::NotRunning)?;
        stream
            .set_read_timeout(Some(CLIENT_READ_TIMEOUT))
            .map_err(ClientError::Io)?;

        let out = stream.try_clone().map_err(ClientError::Io)?;

        Ok(Self {
            out,
            frames: FrameReader::new(stream)
        })
    }

    /// Asks `app.status` and reads the typed answer.
    pub fn app_status(&mut self) -> Result<AppStatusResult, ClientError>
    {
        self.ask(SocketMethod::AppStatus)
    }

    /// Asks `account.inspect` and reads the typed answer.
    ///
    /// The same model the Today screen renders, so that `trdr account inspect`
    /// and the window cannot disagree about what the account holds.
    pub fn account_inspect(&mut self) -> Result<AccountInspectResult, ClientError>
    {
        self.ask(SocketMethod::AccountInspect)
    }

    /// Asks `data.coverage` and reads the typed answer.
    pub fn data_coverage(&mut self) -> Result<DataCoverageResult, ClientError>
    {
        self.ask(SocketMethod::DataCoverage)
    }

    /// One request, one answer, checked.
    ///
    /// Generic over the result so that adding a read is one method naming its
    /// own type rather than a copy of the frame handling. What must not vary is
    /// in here: the id is minted per request, the answer is parsed strictly, and
    /// a result that names a different request is refused rather than returned.
    fn ask<R: serde::de::DeserializeOwned>(
        &mut self,
        method: SocketMethod
    ) -> Result<R, ClientError>
    {
        let name = method.name();
        let request = SocketRequest::new(fresh_request_id(), method);
        let line = request
            .to_json_line()
            .map_err(|_| ClientError::Frame(FrameError::InvalidParams { method: name }))?;

        self.out
            .write_all(line.as_bytes())
            .map_err(|_| ClientError::NotRunning)?;

        let answer = match self.frames.next_frame()
        {
            Ok(Some(answer)) => answer,
            Ok(None) => return Err(ClientError::NoAnswer),
            Err(FrameReadError::Io(error)) => return Err(ClientError::Io(error)),
            Err(_) => return Err(ClientError::NoAnswer)
        };

        let response: SocketResponse<R> =
            SocketResponse::from_json_line(&answer).map_err(ClientError::Frame)?;

        match response.outcome
        {
            // The id is checked on a result and not on a refusal. A frame the
            // server could not read far enough to find an id in still has to be
            // answered, and the protocol has no response without one, so a
            // refusal is allowed to name an id this side never sent.
            SocketOutcome::Ok(result) if response.id == request.id => Ok(result),
            SocketOutcome::Ok(_) => Err(ClientError::MismatchedResponse),
            SocketOutcome::Error(envelope) => Err(ClientError::Refused(Box::new(envelope)))
        }
    }
}

/// A request id for a frame this process is sending.
///
/// The runtime is where a ULID is minted; `trdr-core` can read one but has no
/// clock and no randomness with which to make one.
fn fresh_request_id() -> RequestId
{
    RequestId::from_ulid(Ulid::generate())
}

/// Splits a byte stream into newline-delimited frames.
///
/// Doing this rather than reaching for `BufRead::lines` is not style. `lines`
/// grows its `String` until it meets a newline, so a peer that sends a gigabyte
/// without one is a memory cost this process pays; the buffer here refuses at
/// [`MAX_FRAME_BYTES`], which is the same limit `trdr-core` applies to a frame it
/// has already received. It also has to survive both halves of the same problem:
/// one frame arriving in several reads, and several frames arriving in one.
struct FrameReader<R>
{
    source: R,
    buffer: Vec<u8>,
    finished: bool
}

impl<R: Read> FrameReader<R>
{
    /// Reads frames out of `source`.
    fn new(source: R) -> Self
    {
        Self {
            source,
            buffer: Vec::new(),
            finished: false
        }
    }

    /// The next whole frame, or `None` once the peer has finished cleanly.
    fn next_frame(&mut self) -> Result<Option<String>, FrameReadError>
    {
        loop
        {
            if let Some(end) = self.buffer.iter().position(|byte| *byte == b'\n')
            {
                let mut frame: Vec<u8> = self.buffer.drain(..=end).collect();
                frame.pop();

                return String::from_utf8(frame)
                    .map(Some)
                    .map_err(|_| FrameReadError::NotUtf8);
            }

            if self.buffer.len() > MAX_FRAME_BYTES
            {
                return Err(FrameReadError::TooLarge {
                    limit: MAX_FRAME_BYTES
                });
            }

            if self.finished
            {
                return match self.buffer.is_empty()
                {
                    true => Ok(None),
                    // The protocol is one object per line. Bytes with no newline
                    // after them are half a frame, not a lenient one.
                    false => Err(FrameReadError::Truncated)
                };
            }

            let mut chunk = [0u8; 4096];

            match self.source.read(&mut chunk)
            {
                Ok(0) => self.finished = true,
                Ok(read) => self.buffer.extend_from_slice(&chunk[..read]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted =>
                {}
                Err(error) => return Err(FrameReadError::Io(error))
            }
        }
    }
}

/// A frame could not be read off the stream.
#[derive(Debug, thiserror::Error)]
enum FrameReadError
{
    /// More than [`MAX_FRAME_BYTES`] arrived with no newline in it.
    #[error("a frame passed the {limit} byte limit before it ended")]
    TooLarge
    {
        /// The limit that was passed.
        limit: usize
    },
    /// The stream ended in the middle of a frame.
    #[error("the stream ended in the middle of a frame")]
    Truncated,
    /// The bytes were not text.
    #[error("a frame was not valid UTF-8")]
    NotUtf8,
    /// The read itself failed, which includes the read timing out.
    #[error(transparent)]
    Io(io::Error)
}

/// The uid of the process on the other end.
///
/// `getpeereid` reads what the kernel recorded when the peer connected, so a
/// caller can neither claim an identity nor hand the connection to another user
/// afterwards.
fn peer_uid(stream: &UnixStream) -> io::Result<libc::uid_t>
{
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;

    // SAFETY: the descriptor is owned by `stream` and open for the call, and both
    // out-parameters are live for its duration.
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0
    {
        return Err(io::Error::last_os_error());
    }

    Ok(uid)
}

/// Waits for a descriptor to have something on it, or for the timeout.
fn wait_readable(descriptor: std::os::unix::io::RawFd, timeout: Duration)
{
    let mut watched = libc::pollfd {
        fd: descriptor,
        events: libc::POLLIN,
        revents: 0
    };

    // SAFETY: one initialised `pollfd` is passed with a length of one, and the
    // descriptor is owned by the listener for the whole call.
    unsafe { libc::poll(&mut watched, 1, timeout.as_millis() as libc::c_int) };
}

/// Refuses a socket path that will not fit in a `sockaddr_un`.
fn check_socket_path(path: &Path) -> Result<(), ServerError>
{
    match path.as_os_str().len() > MAX_SOCKET_PATH_BYTES
    {
        true => Err(ServerError::PathTooLong {
            found: path.as_os_str().len(),
            limit: MAX_SOCKET_PATH_BYTES
        }),
        false => Ok(())
    }
}

/// Removes a socket file left behind by an app that is no longer running.
///
/// Takes the lease rather than reading it, so that this cannot be called from
/// anywhere that is not the writer. See the module documentation for why the
/// lease is the right thing to ask.
fn clear_stale_socket(_lease: &WriterLease, socket: &Path) -> Result<(), ServerError>
{
    match std::fs::remove_file(socket)
    {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(ServerError::ClearStaleSocket {
            path: socket.to_path_buf(),
            source
        })
    }
}

/// The socket could not be bound.
#[derive(Debug, thiserror::Error)]
pub enum ServerError
{
    /// The path is longer than a `sockaddr_un` can hold.
    #[error("the socket path is {found} bytes, the limit is {limit}")]
    PathTooLong
    {
        /// How long it was.
        found: usize,
        /// The limit it passed.
        limit: usize
    },
    /// The directories under the product root could not be prepared.
    #[error("the run directory could not be prepared: {source}")]
    Prepare
    {
        /// What went wrong.
        #[source]
        source: Box<crate::root::RootError>
    },
    /// A socket file left by a dead app could not be removed.
    #[error("{path} could not be removed: {source}")]
    ClearStaleSocket
    {
        /// The leftover file.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: io::Error
    },
    /// The socket could not be created or made private.
    #[error("{path} could not be bound: {source}")]
    Bind
    {
        /// The socket path.
        path: PathBuf,
        /// What the operating system said.
        #[source]
        source: io::Error
    }
}

/// Asking a running app did not work.
#[derive(Debug, thiserror::Error)]
pub enum ClientError
{
    /// Nothing is listening, so no app is running.
    #[error("no app is listening on the socket")]
    NotRunning,
    /// The socket path is longer than a `sockaddr_un` can hold.
    #[error("the socket path is {found} bytes, the limit is {limit}")]
    PathTooLong
    {
        /// How long it was.
        found: usize,
        /// The limit it passed.
        limit: usize
    },
    /// The app closed the connection without answering.
    #[error("the app closed the connection without answering")]
    NoAnswer,
    /// The answer belonged to a different request.
    #[error("the answer did not name the request that was sent")]
    MismatchedResponse,
    /// The answer did not obey the protocol.
    #[error(transparent)]
    Frame(FrameError),
    /// The app answered, and the answer was a refusal.
    #[error("the app refused the request")]
    Refused(Box<ErrorEnvelope>),
    /// The connection itself failed.
    #[error(transparent)]
    Io(io::Error)
}

impl ClientError
{
    /// The same failure in the one shape every surface renders (section 12).
    ///
    /// Everything that means "there is no app to ask" becomes `APP_NOT_RUNNING`
    /// and says which one it was in a parameter, the way `trdr-core` already
    /// distinguishes its seven frame refusals under one code. No sentence crosses
    /// this boundary; the words belong to whichever surface is speaking.
    pub fn to_envelope(&self) -> ErrorEnvelope
    {
        match self
        {
            Self::Refused(envelope) => (**envelope).clone(),
            Self::Frame(refusal) => refusal.to_envelope(None),
            Self::NotRunning => not_running("no_socket"),
            Self::Io(_) => not_running("connection_failed"),
            Self::NoAnswer => not_running("no_answer"),
            Self::PathTooLong { found, limit } => not_running("socket_path_too_long")
                .with_param("found", ErrorParam::Integer(*found as i64))
                .with_param("limit", ErrorParam::Integer(*limit as i64)),
            Self::MismatchedResponse => ErrorEnvelope::new(ErrorCode::AppProtocolVersion)
                .with_param("reason", ErrorParam::literal("response_id_mismatch"))
        }
    }
}

/// `APP_NOT_RUNNING`, saying which way the app turned out not to be there.
fn not_running(reason: &'static str) -> ErrorEnvelope
{
    ErrorEnvelope::new(ErrorCode::AppNotRunning)
        .with_param("reason", ErrorParam::literal(reason))
        .with_retryability(Retryability::No)
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn a_frame_split_across_reads_is_put_back_together()
    {
        let pieces: Vec<&[u8]> = vec![b"{\"v\":1,", b"\"id\":\"x\"}", b"\n"];
        let mut reader = FrameReader::new(Trickle::new(pieces));

        assert_eq!(
            reader.next_frame().unwrap().as_deref(),
            Some("{\"v\":1,\"id\":\"x\"}")
        );
        assert_eq!(reader.next_frame().unwrap(), None);
    }

    #[test]
    fn several_frames_arriving_in_one_read_are_taken_apart()
    {
        let mut reader = FrameReader::new(Trickle::new(vec![b"one\ntwo\nthree\n"]));

        assert_eq!(reader.next_frame().unwrap().as_deref(), Some("one"));
        assert_eq!(reader.next_frame().unwrap().as_deref(), Some("two"));
        assert_eq!(reader.next_frame().unwrap().as_deref(), Some("three"));
        assert_eq!(reader.next_frame().unwrap(), None);
    }

    #[test]
    fn a_carriage_return_is_left_for_the_parser_to_trim()
    {
        let mut reader = FrameReader::new(Trickle::new(vec![b"one\r\n"]));

        assert_eq!(reader.next_frame().unwrap().as_deref(), Some("one\r"));
    }

    #[test]
    fn a_stream_that_stops_mid_frame_is_not_treated_as_a_frame()
    {
        let mut reader = FrameReader::new(Trickle::new(vec![b"{\"v\":1}"]));

        assert!(matches!(
            reader.next_frame(),
            Err(FrameReadError::Truncated)
        ));
    }

    // The reason this reader exists rather than `BufRead::lines`: the limit has
    // to bite while the bytes are arriving, not after a whole line has been kept.
    #[test]
    fn bytes_with_no_newline_are_refused_at_the_frame_limit()
    {
        let mut reader = FrameReader::new(Flood);

        assert!(matches!(
            reader.next_frame(),
            Err(FrameReadError::TooLarge { .. })
        ));
    }

    #[test]
    fn a_frame_that_is_not_text_is_refused()
    {
        let mut reader = FrameReader::new(Trickle::new(vec![b"\xff\xfe\n"]));

        assert!(matches!(reader.next_frame(), Err(FrameReadError::NotUtf8)));
    }

    #[test]
    fn this_process_is_recognised_as_the_owner_of_its_own_connection()
    {
        let (here, there) = UnixStream::pair().expect("a socket pair could not be made");

        // SAFETY: `getuid` reads this process's own identity and cannot fail.
        let mine = unsafe { libc::getuid() };

        assert_eq!(peer_uid(&here).unwrap(), mine);
        assert_eq!(peer_uid(&there).unwrap(), mine);
    }

    // The rejection branch itself, not just the uid reader. The expected owner is
    // set to a uid this process does not have, which is the only way to reach the
    // branch without a second user account.
    #[test]
    fn a_connection_from_another_user_is_closed_without_an_answer()
    {
        // SAFETY: `getuid` reads this process's own identity and cannot fail.
        let mine = unsafe { libc::getuid() };
        let served = Arc::new(Served {
            bridge: Arc::new(FixedStatus),
            owner_uid: mine.wrapping_add(1)
        });

        let (server_side, mut client_side) =
            UnixStream::pair().expect("a socket pair could not be made");
        // Set before the peer goes away: a socket whose other end has closed no
        // longer accepts its options being changed.
        client_side
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("the read timeout could not be set");

        let worker = thread::spawn(move || served.converse(server_side));
        let _ = client_side.write_all(
            SocketRequest::new(fresh_request_id(), SocketMethod::AppStatus)
                .to_json_line()
                .unwrap()
                .as_bytes()
        );

        worker.join().expect("the connection thread panicked");

        let mut answer = Vec::new();
        let _ = client_side.read_to_end(&mut answer);

        assert!(
            answer.is_empty(),
            "a foreign uid was told something: {answer:?}"
        );
    }

    #[test]
    fn a_connection_from_this_user_is_answered()
    {
        // SAFETY: `getuid` reads this process's own identity and cannot fail.
        let served = Arc::new(Served {
            bridge: Arc::new(FixedStatus),
            owner_uid: unsafe { libc::getuid() }
        });

        let (server_side, client_side) =
            UnixStream::pair().expect("a socket pair could not be made");
        thread::spawn(move || served.converse(server_side));

        let mut out = client_side.try_clone().unwrap();
        let request = SocketRequest::new(fresh_request_id(), SocketMethod::AppStatus);
        out.write_all(request.to_json_line().unwrap().as_bytes())
            .unwrap();

        let mut frames = FrameReader::new(client_side);
        let line = frames
            .next_frame()
            .unwrap()
            .expect("an answer was expected");
        let response = SocketResponse::<AppStatusResult>::from_json_line(&line).unwrap();

        assert_eq!(response.id, request.id);
        assert!(matches!(response.outcome, SocketOutcome::Ok(_)));
    }

    #[test]
    fn a_method_this_build_does_not_serve_is_refused_by_name()
    {
        let served = Served {
            bridge: Arc::new(FixedStatus),
            // SAFETY: `getuid` reads this process's own identity and cannot fail.
            owner_uid: unsafe { libc::getuid() }
        };

        let request = SocketRequest::new(fresh_request_id(), SocketMethod::UiFocus);
        let response = served.answer(request.to_json_line().unwrap().trim_end());

        match response.outcome
        {
            SocketOutcome::Error(envelope) =>
            {
                assert_eq!(envelope.code, ErrorCode::AppProtocolVersion);
                assert_eq!(
                    envelope.params.get("method"),
                    Some(&ErrorParam::literal("ui.focus"))
                );
            }
            SocketOutcome::Ok(_) => panic!("a method this build has not built answered")
        }
    }

    #[test]
    fn a_frame_this_build_cannot_read_is_refused_rather_than_ignored()
    {
        let served = Served {
            bridge: Arc::new(FixedStatus),
            // SAFETY: `getuid` reads this process's own identity and cannot fail.
            owner_uid: unsafe { libc::getuid() }
        };

        for line in [
            "{",
            "{\"v\":9,\"id\":\"01KZNNR5X818P3J6ENYKSADP8W\",\"method\":\"app.status\"}"
        ]
        {
            match served.answer(line).outcome
            {
                SocketOutcome::Error(envelope) =>
                {
                    assert_eq!(envelope.code, ErrorCode::AppProtocolVersion)
                }
                SocketOutcome::Ok(_) => panic!("`{line}` was accepted")
            }
        }
    }

    use crate::clock::SystemClock;
    use crate::query::{QueryService, SyntheticQueries};

    /// A bridge with nothing behind it, for the protocol tests.
    struct FixedStatus;

    impl AppBridge for FixedStatus
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
                product_root: PathBuf::from("/private/tmp/t"),
                workspace_path: PathBuf::from("/private/tmp/t/workspaces/default"),
                schema_version: 1
            }
        }
    }

    #[test]
    fn a_path_a_sockaddr_cannot_hold_is_refused_as_itself()
    {
        let long = PathBuf::from("/private/tmp").join("x".repeat(MAX_SOCKET_PATH_BYTES));

        assert!(matches!(
            check_socket_path(&long),
            Err(ServerError::PathTooLong { limit, .. }) if limit == MAX_SOCKET_PATH_BYTES
        ));
        assert!(check_socket_path(Path::new("/private/tmp/t/run/app.sock")).is_ok());
    }

    #[test]
    fn every_way_of_not_reaching_an_app_renders_as_one_code()
    {
        for error in [
            ClientError::NotRunning,
            ClientError::NoAnswer,
            ClientError::Io(io::Error::from(io::ErrorKind::BrokenPipe)),
            ClientError::PathTooLong {
                found: 200,
                limit: MAX_SOCKET_PATH_BYTES
            }
        ]
        {
            let envelope = error.to_envelope();

            assert_eq!(envelope.code, ErrorCode::AppNotRunning);
            assert!(envelope.params.contains_key("reason"));
        }
    }

    #[test]
    fn a_refusal_from_the_app_is_passed_through_unchanged()
    {
        let envelope = ErrorEnvelope::new(ErrorCode::DbBusy);
        let error = ClientError::Refused(Box::new(envelope.clone()));

        assert_eq!(error.to_envelope(), envelope);
    }

    /// A reader that hands over one prepared piece per call, so a test can choose
    /// how a frame is split.
    struct Trickle
    {
        pieces: std::collections::VecDeque<Vec<u8>>
    }

    impl Trickle
    {
        fn new(pieces: Vec<&[u8]>) -> Self
        {
            Self {
                pieces: pieces.into_iter().map(<[u8]>::to_vec).collect()
            }
        }
    }

    impl Read for Trickle
    {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize>
        {
            let Some(piece) = self.pieces.pop_front()
            else
            {
                return Ok(0);
            };

            let taken = piece.len().min(out.len());
            out[..taken].copy_from_slice(&piece[..taken]);

            if taken < piece.len()
            {
                self.pieces.push_front(piece[taken..].to_vec());
            }

            Ok(taken)
        }
    }

    /// A reader with no end and no newline in it.
    struct Flood;

    impl Read for Flood
    {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize>
        {
            out.fill(b'x');
            Ok(out.len())
        }
    }
}
