//! The agent's terminal: one child process under a pseudo-terminal.
//!
//! What lands here (`docs/FOUNDATION_DESIGN.md` sections 3 and 9.1): spawning
//! one Claude Code or Codex child under a pseudo-terminal, owning it, and moving
//! bytes both ways.
//!
//! The bytes are bytes. Section 1 is explicit that agent output is shown as a
//! raw stream and is never read as product state, and section 11 forbids
//! converting between terminal content and a query model in either direction.
//! Nothing in this module looks at a byte it carries — [`TerminalSession`] hands
//! the stream to a [`TerminalSink`] it knows nothing about, and the only thing
//! it does with a chunk besides forwarding it is append it to
//! [`crate::scrollback`] as itself.
//!
//! # Two channels, and why the split is structural
//!
//! Section 9.1's last bullet keeps host-to-UI state events separate from the
//! terminal byte stream. [`TerminalSink`] has two methods for that reason:
//! [`TerminalSink::output`] carries bytes and nothing else, and
//! [`TerminalSink::lifecycle`] carries a [`PtyLifecycle`] and nothing else. A
//! state that wanted to say what the agent printed would have nowhere to put it,
//! which is a stronger guarantee than a rule about what to put where.
//!
//! # What this module can and cannot say about the process
//!
//! [`PtyLifecycle`] has five values, and the visual contract's terminal has
//! seven. The missing two are missing on purpose. `needs-input` cannot be
//! detected without reading the stream for prompts, which is the thing section
//! 11 forbids; `approval-pending` belongs to the registration approval
//! round-trip, which is milestone M6. Neither is expressible here, so no code on
//! this path can produce one by accident. Where the remaining two states come
//! from in M2 is a question for the surface, not for the runtime.
//!
//! # Backpressure
//!
//! There is no queue between the child and the sink, which is what bounds it.
//! The reader thread calls [`TerminalSink::output`] synchronously, so a sink
//! that is slow stops the reader, a reader that has stopped stops draining the
//! pseudo-terminal, and a full pseudo-terminal blocks the child's own `write`.
//! That is the operating system's own backpressure, and it means nothing here
//! ever has to decide which bytes to drop — a terminal that dropped bytes would
//! be a terminal that renders a screen the agent never drew. The two bounds that
//! do exist are on the ends: [`crate::scrollback::KEEP_BYTES`] on the host's
//! side, and the WebView terminal's own scrollback on the other.

use crate::env;
use crate::scrollback::Scrollback;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};

/// How many bytes one read hands over at most.
///
/// Sized for the boundary it crosses rather than for the pipe: base64 of this
/// many bytes, inside a JSON message, stays under the size at which Tauri stops
/// handing a channel message straight to the WebView and starts routing it
/// through a second round trip.
const MAX_CHUNK_BYTES: usize = 4096;

/// The window size a terminal is spawned at and resized to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtyWindow
{
    /// Columns.
    pub cols: u16,
    /// Rows.
    pub rows: u16
}

impl PtyWindow
{
    /// The size a terminal starts at before anything has measured one.
    ///
    /// Never what the screen actually has. It is what the child would be told if
    /// it were spawned before the WebView had laid its terminal out, and the
    /// point of [`TerminalSession::resize`] accepting a size with no process
    /// running is that this value is replaced before a child ever sees it.
    pub const DEFAULT: Self = Self { cols: 80, rows: 24 };
}

impl Default for PtyWindow
{
    fn default() -> Self
    {
        Self::DEFAULT
    }
}

/// Which program to run, and where.
///
/// Never built from anything the WebView sent. Section 9.1 puts the agent
/// executable in a setting the host owns, so a screen can ask for the terminal
/// to start and cannot ask for a particular program to be the terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyRequest
{
    /// The program, as a bare name to look up or a location to use as given.
    pub program: String,
    /// Its arguments.
    pub args: Vec<String>,
    /// The directory it starts in.
    pub cwd: Option<PathBuf>
}

/// Where a terminal's process is in its life.
///
/// Five values, and section 12's terminal states are seven; see the module
/// documentation for why the other two cannot be produced here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PtyLifecycle
{
    /// A start has been asked for and no child exists yet.
    Starting,
    /// A child is running and has printed nothing.
    Ready,
    /// The child has printed something.
    ///
    /// The *fact* that output arrived, never what it was. This is the only thing
    /// on this path that the byte stream influences at all, and it influences it
    /// by existing rather than by saying anything — the same state results
    /// whatever the bytes hold, which [`tests`] checks.
    Running,
    /// The previous child is being stopped and a new one started.
    Reconnecting,
    /// No child is running.
    Exited
}

impl PtyLifecycle
{
    /// How far through one spawn's life this state is.
    ///
    /// Two threads move one spawn along — the one that asked for the start, and
    /// the one reading the child — and they never agree on an order first. A
    /// child that prints and exits inside a millisecond can be finished before
    /// the thread that spawned it has finished writing `ready` down. Ordering
    /// the states and refusing to move backwards is what makes that harmless
    /// instead of leaving a session that says `ready` about a child that has
    /// already gone.
    ///
    /// A new spawn starts the ordering again; see [`TerminalSession::spawn`].
    pub const fn step(self) -> u8
    {
        match self
        {
            Self::Reconnecting => 0,
            Self::Starting => 1,
            Self::Ready => 2,
            Self::Running => 3,
            Self::Exited => 4
        }
    }
}

/// Where a terminal's bytes and its process state go.
///
/// Two methods, two channels, and neither can carry the other's payload. The
/// surface implements this; nothing here knows what happens afterwards, which is
/// what keeps "the app must not parse the stream" a property of the code rather
/// than a promise in a comment.
pub trait TerminalSink: Send + Sync
{
    /// Bytes from the child, in the order they arrived, unaltered.
    fn output(&self, bytes: &[u8]);

    /// The process state changed. Carries no terminal content, ever.
    fn lifecycle(&self, state: PtyLifecycle);
}

/// What a spawned child's reader reports back to whoever opened it.
///
/// Separate from [`TerminalSink`] because a host does not know about sessions:
/// it reads a child and says what it read, and [`TerminalSession`] is what
/// decides whether those bytes are still wanted.
pub trait ChildSink: Send + Sync
{
    /// The child wrote these bytes.
    fn wrote(&self, bytes: &[u8]);

    /// The child's output ended, which is how a pseudo-terminal reports an exit.
    fn ended(&self);
}

/// Spawning a child under a pseudo-terminal — the seam section 13 calls
/// `PtyHost`.
///
/// One implementation ([`SystemPtyHost`]) spawns a real process; a test
/// substitutes its own and never needs an agent CLI installed.
pub trait PtyHost: Send + Sync
{
    /// Starts the program, sized to the window, reading into the sink.
    fn open(
        &self,
        request: &PtyRequest,
        window: PtyWindow,
        sink: Arc<dyn ChildSink>
    ) -> Result<Box<dyn PtyProcess>, TerminalError>;
}

/// A running child, from the outside.
///
/// Every method is `&mut`, and every call happens while [`TerminalSession`]
/// holds its own lock or owns the value outright. That is what keeps the writer
/// and the window from needing locks of their own.
pub trait PtyProcess: Send
{
    /// Sends bytes to the child.
    fn write(&mut self, bytes: &[u8]) -> Result<(), TerminalError>;

    /// Tells the child the window changed size.
    fn resize(&mut self, window: PtyWindow) -> Result<(), TerminalError>;

    /// The child's process id, where the system gave one.
    fn pid(&self) -> Option<u32>;

    /// Stops the child and waits for it.
    ///
    /// Waiting is the part that matters: a killed child that is never waited on
    /// stays as a zombie for as long as this process runs, and a terminal that
    /// is restarted a few times a day would collect them.
    fn reap(&mut self);
}

/// The real thing: `portable-pty` over the operating system's pseudo-terminals.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemPtyHost;

impl PtyHost for SystemPtyHost
{
    fn open(
        &self,
        request: &PtyRequest,
        window: PtyWindow,
        sink: Arc<dyn ChildSink>
    ) -> Result<Box<dyn PtyProcess>, TerminalError>
    {
        let program = env::resolve_program(&request.program).ok_or_else(|| {
            TerminalError::ExecutableMissing {
                name: request.program.clone()
            }
        })?;

        let pair = native_pty_system()
            .openpty(size_of(window))
            .map_err(|_| TerminalError::Spawn)?;
        let child = pair
            .slave
            .spawn_command(command(&program, request))
            .map_err(|_| TerminalError::Spawn)?;

        // The slave end is the child's now. Holding a copy would keep the
        // pseudo-terminal open after the child exits, and the reader below would
        // wait forever for an end that never comes.
        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|_| TerminalError::Spawn)?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|_| TerminalError::Spawn)?;

        pump(reader, sink);

        Ok(Box::new(SystemPtyProcess {
            master: pair.master,
            writer,
            child
        }))
    }
}

/// The command the child is started with.
///
/// The child is given the login shell's `PATH`, not the app's. Without it a
/// resolved `claude` starts and then cannot find the tools it shells out to,
/// which looks like a broken agent rather than a broken environment.
fn command(program: &std::path::Path, request: &PtyRequest) -> CommandBuilder
{
    let mut command = CommandBuilder::new(program);

    for argument in &request.args
    {
        command.arg(argument);
    }

    if let Some(directory) = &request.cwd
    {
        command.cwd(directory);
    }

    command.env("TERM", "xterm-256color");

    if let Some(path) = env::login_path()
    {
        command.env("PATH", path);
    }

    command
}

/// The window, in the shape `portable-pty` wants it.
fn size_of(window: PtyWindow) -> PtySize
{
    PtySize {
        rows: window.rows,
        cols: window.cols,
        pixel_width: 0,
        pixel_height: 0
    }
}

/// Reads the child until it ends, on a thread of its own.
///
/// The thread ends when the read ends, which happens when the child's last
/// descriptor on the pseudo-terminal closes. There is nothing to cancel: a
/// session that no longer wants these bytes drops them on arrival, and the
/// thread goes away when the process it is reading does.
fn pump(mut reader: Box<dyn Read + Send>, sink: Arc<dyn ChildSink>)
{
    std::thread::spawn(move || {
        let mut buffer = [0u8; MAX_CHUNK_BYTES];

        loop
        {
            match reader.read(&mut buffer)
            {
                Ok(0) => break,
                Ok(count) => sink.wrote(&buffer[..count]),
                Err(_) => break
            }
        }

        sink.ended();
    });
}

/// One child, its pseudo-terminal, and the two ends of it this process holds.
struct SystemPtyProcess
{
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>
}

impl PtyProcess for SystemPtyProcess
{
    fn write(&mut self, bytes: &[u8]) -> Result<(), TerminalError>
    {
        self.writer
            .write_all(bytes)
            .and_then(|()| self.writer.flush())
            .map_err(|_| TerminalError::NotRunning)
    }

    fn resize(&mut self, window: PtyWindow) -> Result<(), TerminalError>
    {
        self.master
            .resize(size_of(window))
            .map_err(|_| TerminalError::NotRunning)
    }

    fn pid(&self) -> Option<u32>
    {
        self.child.process_id()
    }

    fn reap(&mut self)
    {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// What a start or a restart settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalStarted
{
    /// The child's process id, where the system gave one.
    pub pid: Option<u32>,
    /// The size the child was told about.
    pub window: PtyWindow
}

/// The app's one agent terminal.
///
/// Holds at most one child at a time. Terminal tabs and concurrent sessions are
/// outside M2, and the shape says so: there is one process, one window, and one
/// sink.
pub struct TerminalSession
{
    host: Arc<dyn PtyHost>,
    scrollback: Scrollback,
    state: Mutex<SessionState>
}

/// Everything about the session that changes.
struct SessionState
{
    process: Option<Box<dyn PtyProcess>>,
    sink: Option<Arc<dyn TerminalSink>>,
    /// Which spawn is the current one.
    ///
    /// A restart raises this before the old child is stopped, so bytes still in
    /// flight from that child arrive carrying a number that is no longer
    /// current and are dropped rather than interleaved with the new child's
    /// first output.
    generation: u64,
    window: PtyWindow,
    lifecycle: PtyLifecycle
}

impl TerminalSession
{
    /// A session that has never started anything.
    pub fn new(host: Arc<dyn PtyHost>, scrollback: Scrollback) -> Arc<Self>
    {
        Arc::new(Self {
            host,
            scrollback,
            state: Mutex::new(SessionState {
                process: None,
                sink: None,
                generation: 0,
                window: PtyWindow::DEFAULT,
                lifecycle: PtyLifecycle::Exited
            })
        })
    }

    /// Points the session at a surface, and replays what is already scrolled
    /// back.
    ///
    /// Called once per WebView, which means once per app run and again after a
    /// reload. Replaying first and then reporting the state is the order that
    /// leaves a fresh terminal showing the session as it is rather than as it
    /// started.
    pub fn attach(&self, sink: Arc<dyn TerminalSink>)
    {
        let saved = self.scrollback.load();

        let lifecycle = {
            let mut state = self.locked();
            state.sink = Some(Arc::clone(&sink));
            state.lifecycle
        };

        if !saved.is_empty()
        {
            sink.output(&saved);
        }

        sink.lifecycle(lifecycle);
    }

    /// Starts the agent, or answers with the one already running.
    ///
    /// Starting twice is not an error and does not produce a second child. A
    /// WebView that reloaded has no way to know whether the host still has a
    /// terminal, and answering "here is the one you have" is the only response
    /// that leaves the two sides agreeing.
    pub fn start(self: &Arc<Self>, request: &PtyRequest) -> Result<TerminalStarted, TerminalError>
    {
        let window = {
            let state = self.locked();

            if let Some(process) = state.process.as_ref()
            {
                return Ok(TerminalStarted {
                    pid: process.pid(),
                    window: state.window
                });
            }

            state.window
        };

        self.spawn(request, window)
    }

    /// Stops the agent and starts it again.
    ///
    /// The old child is stopped and waited for before the new one is asked for,
    /// so there is never a moment with two children on one terminal.
    pub fn restart(self: &Arc<Self>, request: &PtyRequest)
        -> Result<TerminalStarted, TerminalError>
    {
        let (previous, window) = {
            let mut state = self.locked();
            state.generation += 1;
            state.lifecycle = PtyLifecycle::Reconnecting;
            (state.process.take(), state.window)
        };

        self.publish(PtyLifecycle::Reconnecting);

        // Outside the lock: waiting for a child to die takes as long as the
        // child takes, and holding the session's lock across it would stop the
        // surface from being told anything in the meantime.
        if let Some(mut process) = previous
        {
            process.reap();
        }

        self.spawn(request, window)
    }

    /// Sends bytes to the agent.
    pub fn input(&self, bytes: &[u8]) -> Result<(), TerminalError>
    {
        self.locked()
            .process
            .as_mut()
            .ok_or(TerminalError::NotRunning)?
            .write(bytes)
    }

    /// Records the window size, and tells the child if there is one.
    ///
    /// Accepting a size with nothing running is the point rather than a
    /// leniency. The WebView measures its terminal as it lays out, which happens
    /// before `start` has returned and again after the child has exited, and
    /// both of those are the correct size — they are simply not deliverable yet.
    /// Keeping the last one means the next child is spawned at the size the
    /// screen already has, instead of at a guess that a resize then corrects one
    /// frame later.
    pub fn resize(&self, window: PtyWindow) -> Result<(), TerminalError>
    {
        let mut state = self.locked();
        state.window = window;

        match state.process.as_mut()
        {
            Some(process) => process.resize(window),
            None => Ok(())
        }
    }

    /// Where the process is now.
    pub fn lifecycle(&self) -> PtyLifecycle
    {
        self.locked().lifecycle
    }

    /// The window the next child would be spawned at.
    pub fn window(&self) -> PtyWindow
    {
        self.locked().window
    }

    /// The child's process id, if one is running.
    pub fn pid(&self) -> Option<u32>
    {
        self.locked().process.as_ref().and_then(|child| child.pid())
    }

    /// Spawns, and moves the state through `starting` to `ready` or back to
    /// `exited`.
    fn spawn(
        self: &Arc<Self>,
        request: &PtyRequest,
        window: PtyWindow
    ) -> Result<TerminalStarted, TerminalError>
    {
        let generation = {
            let mut state = self.locked();
            state.generation += 1;
            state.lifecycle = PtyLifecycle::Starting;
            state.generation
        };

        self.publish(PtyLifecycle::Starting);

        let opened = self.host.open(
            request,
            window,
            Arc::new(Stream {
                session: Arc::downgrade(self),
                generation
            })
        );

        let mut process = match opened
        {
            Ok(process) => process,
            Err(error) =>
            {
                self.advance(generation, PtyLifecycle::Exited);
                return Err(error);
            }
        };

        // One lock for the size and the hand-over together, because the gap
        // between them is the race. The screen measures its terminal while the
        // shell is laying out, which is exactly while a child is starting: a
        // resize that landed after the size was read and before the process was
        // stored would find no process to tell and then be overwritten by a
        // spawn that had already read the old size.
        //
        // A `SIGWINCH` a child has not installed a handler for yet is harmless;
        // a child left at the wrong size draws a first frame that is already
        // wrong.
        let started = {
            let mut state = self.locked();

            // A child the reader has already reported the end of is not stored.
            // Two threads write this state, and that is the ordering that has to
            // hold: no session holds a process that has gone.
            if state.generation != generation || state.lifecycle == PtyLifecycle::Exited
            {
                TerminalStarted {
                    pid: process.pid(),
                    window: state.window
                }
            }
            else
            {
                if state.window != window
                {
                    let _ = process.resize(state.window);
                }

                let started = TerminalStarted {
                    pid: process.pid(),
                    window: state.window
                };
                state.process = Some(process);
                started
            }
        };

        self.advance(generation, PtyLifecycle::Ready);

        Ok(started)
    }

    /// What the reader thread does with a chunk.
    ///
    /// The generation check and the scrollback write happen under the lock; the
    /// sink calls happen after it is released. The sink is the surface, and a
    /// surface is allowed to call back into this session — a lock held across
    /// that call is a deadlock waiting for the first person who tries it.
    fn deliver(&self, generation: u64, bytes: &[u8])
    {
        let sink = {
            let state = self.locked();

            if state.generation != generation
            {
                return;
            }

            self.scrollback.append(bytes);
            state.sink.clone()
        };

        if let Some(sink) = sink
        {
            sink.output(bytes);
        }

        // Said after the bytes, and said about the fact that they arrived rather
        // than about what they were.
        self.advance(generation, PtyLifecycle::Running);
    }

    /// What the reader thread does when the child's output ends.
    fn finish(&self, generation: u64)
    {
        self.advance(generation, PtyLifecycle::Exited);
    }

    /// Records a state for this spawn and tells the surface, if it is a step
    /// forward and this spawn is still the current one.
    ///
    /// Everything that moves the lifecycle after a spawn has begun goes through
    /// here, so "forward only, current spawn only" is one rule in one place
    /// rather than a condition repeated at four call sites.
    fn advance(&self, generation: u64, to: PtyLifecycle)
    {
        let sink = {
            let mut state = self.locked();

            if state.generation != generation || to.step() <= state.lifecycle.step()
            {
                return;
            }

            state.lifecycle = to;

            if to == PtyLifecycle::Exited
            {
                state.process = None;
            }

            state.sink.clone()
        };

        if let Some(sink) = sink
        {
            sink.lifecycle(to);
        }
    }

    /// Tells the surface about a state the caller has already recorded.
    fn publish(&self, lifecycle: PtyLifecycle)
    {
        let sink = self.locked().sink.clone();

        if let Some(sink) = sink
        {
            sink.lifecycle(lifecycle);
        }
    }

    /// The state, with a poisoned lock recovered rather than propagated.
    ///
    /// A panic somewhere else is not a reason for the terminal to stop
    /// answering: the state behind this lock is a handful of small values with
    /// no invariant spanning them, so the worst a poisoned lock can mean here is
    /// that one of them is stale.
    fn locked(&self) -> std::sync::MutexGuard<'_, SessionState>
    {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// One spawn's reader, as the host sees it.
///
/// Weak, so that a session dropped while a child is still printing lets the
/// reader thread find nothing and stop, rather than keeping the session alive
/// through the child.
struct Stream
{
    session: Weak<TerminalSession>,
    generation: u64
}

impl ChildSink for Stream
{
    fn wrote(&self, bytes: &[u8])
    {
        if let Some(session) = self.session.upgrade()
        {
            session.deliver(self.generation, bytes);
        }
    }

    fn ended(&self)
    {
        if let Some(session) = self.session.upgrade()
        {
            session.finish(self.generation);
        }
    }
}

/// What can go wrong with the agent terminal (section 12's `TERMINAL_*`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TerminalError
{
    /// The program is not on the login shell's `PATH` and is not a file.
    #[error("`{name}` is not an executable on the login shell's PATH")]
    ExecutableMissing
    {
        /// The name that was asked for. The configured name, never a path this
        /// process resolved, so an error cannot describe the user's filesystem.
        name: String
    },
    /// The pseudo-terminal or the child could not be started.
    #[error("the agent process could not be started")]
    Spawn,
    /// There is no child to send this to.
    #[error("no agent process is running")]
    NotRunning
}

#[cfg(test)]
mod tests
{
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    /// How long a test waits for a child to say something before giving up.
    const PATIENCE: Duration = Duration::from_secs(10);

    /// A sink that keeps everything it was told, so an assertion can name it.
    #[derive(Default)]
    struct Recorder
    {
        bytes: Mutex<Vec<u8>>,
        states: Mutex<Vec<PtyLifecycle>>,
        delay: Option<Duration>
    }

    impl Recorder
    {
        fn slow(delay: Duration) -> Self
        {
            Self {
                delay: Some(delay),
                ..Self::default()
            }
        }

        fn seen(&self) -> Vec<u8>
        {
            self.bytes.lock().unwrap().clone()
        }

        fn states(&self) -> Vec<PtyLifecycle>
        {
            self.states.lock().unwrap().clone()
        }

        /// Every state since the one `attach` reported.
        ///
        /// Attaching says what the session already was, which for a fresh
        /// session is `exited`. A test waiting for a run to end would otherwise
        /// be satisfied before the run had started.
        fn run_states(&self) -> Vec<PtyLifecycle>
        {
            self.states().into_iter().skip(1).collect()
        }

        fn holds(&self, needle: &[u8]) -> bool
        {
            self.seen()
                .windows(needle.len())
                .any(|window| window == needle)
        }

        fn wait_for(&self, needle: &[u8])
        {
            let deadline = Instant::now() + PATIENCE;

            while Instant::now() < deadline
            {
                if self.holds(needle)
                {
                    return;
                }

                std::thread::sleep(Duration::from_millis(10));
            }

            panic!(
                "never saw {:?} — got {:?}",
                String::from_utf8_lossy(needle),
                String::from_utf8_lossy(&self.seen())
            );
        }

        fn wait_for_state(&self, wanted: PtyLifecycle)
        {
            let deadline = Instant::now() + PATIENCE;

            while Instant::now() < deadline
            {
                if self.run_states().contains(&wanted)
                {
                    return;
                }

                std::thread::sleep(Duration::from_millis(10));
            }

            panic!("never reached {wanted:?} — got {:?}", self.states());
        }
    }

    impl TerminalSink for Recorder
    {
        fn output(&self, bytes: &[u8])
        {
            if let Some(delay) = self.delay
            {
                std::thread::sleep(delay);
            }

            self.bytes.lock().unwrap().extend_from_slice(bytes);
        }

        fn lifecycle(&self, state: PtyLifecycle)
        {
            self.states.lock().unwrap().push(state);
        }
    }

    /// A host that opens nothing, and remembers what it was asked to do.
    ///
    /// The other half of the seam section 13 calls `PtyHost`. What it buys is
    /// not speed but observability: reaping a child and dropping a replaced
    /// child's last bytes are both things a real process can only be asked about
    /// by racing it.
    #[derive(Default)]
    struct RecordingHost
    {
        streams: Mutex<Vec<Arc<dyn ChildSink>>>,
        reaped: Arc<AtomicU64>
    }

    impl RecordingHost
    {
        /// The reader end of the nth child this host opened.
        fn stream(&self, index: usize) -> Arc<dyn ChildSink>
        {
            Arc::clone(&self.streams.lock().unwrap()[index])
        }

        /// How many children have been stopped and waited for.
        fn reaped(&self) -> u64
        {
            self.reaped.load(Ordering::Relaxed)
        }
    }

    impl PtyHost for RecordingHost
    {
        fn open(
            &self,
            _request: &PtyRequest,
            _window: PtyWindow,
            sink: Arc<dyn ChildSink>
        ) -> Result<Box<dyn PtyProcess>, TerminalError>
        {
            let mut streams = self.streams.lock().unwrap();
            streams.push(sink);

            Ok(Box::new(RecordingProcess {
                pid: streams.len() as u32,
                reaped: Arc::clone(&self.reaped)
            }))
        }
    }

    /// A child that does nothing, and says when it was reaped.
    struct RecordingProcess
    {
        pid: u32,
        reaped: Arc<AtomicU64>
    }

    impl PtyProcess for RecordingProcess
    {
        fn write(&mut self, _bytes: &[u8]) -> Result<(), TerminalError>
        {
            Ok(())
        }

        fn resize(&mut self, _window: PtyWindow) -> Result<(), TerminalError>
        {
            Ok(())
        }

        fn pid(&self) -> Option<u32>
        {
            Some(self.pid)
        }

        fn reap(&mut self)
        {
            self.reaped.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn scratch(name: &str) -> Scrollback
    {
        static SERIAL: AtomicU64 = AtomicU64::new(0);

        let directory = PathBuf::from("/private/tmp/trdr-t").join(format!(
            "pty-{name}-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&directory);

        Scrollback::at(directory.join("run/terminal/scrollback.bin"))
    }

    /// Absolute paths, and programs every macOS has. A terminal test that needed
    /// an agent CLI installed would be a test of the machine it ran on.
    fn request(program: &str, args: &[&str]) -> PtyRequest
    {
        PtyRequest {
            program: program.to_owned(),
            args: args.iter().map(|argument| (*argument).to_owned()).collect(),
            cwd: None
        }
    }

    fn session(name: &str) -> Arc<TerminalSession>
    {
        TerminalSession::new(Arc::new(SystemPtyHost), scratch(name))
    }

    fn attached(name: &str) -> (Arc<TerminalSession>, Arc<Recorder>)
    {
        let session = session(name);
        let recorder = Arc::new(Recorder::default());
        session.attach(Arc::clone(&recorder) as Arc<dyn TerminalSink>);

        (session, recorder)
    }

    #[test]
    fn a_child_process_output_reaches_the_sink()
    {
        let (session, recorder) = attached("output");
        let started = session
            .start(&request("/bin/echo", &["from-the-pty"]))
            .expect("echo should start");

        assert!(started.pid.is_some());
        recorder.wait_for(b"from-the-pty");
    }

    /// The property the whole rail rests on: escape sequences arrive as the
    /// bytes the child emitted, not as anything the host decided they meant.
    #[test]
    fn ansi_escapes_and_true_colour_arrive_as_raw_bytes()
    {
        let (session, recorder) = attached("ansi");
        session
            .start(&request(
                "/bin/sh",
                &[
                    "-c",
                    "printf '\\033[31mred\\033[38;2;12;34;56mtrue\\033[0m\\n'"
                ]
            ))
            .expect("sh should start");

        recorder.wait_for(b"true");

        assert!(
            recorder.holds(&[0x1b, b'[', b'3', b'1', b'm']),
            "the CSI sequence was rewritten"
        );
        assert!(
            recorder.holds(b"\x1b[38;2;12;34;56m"),
            "the true-colour sequence was rewritten"
        );
    }

    #[test]
    fn typed_input_reaches_the_child()
    {
        let (session, recorder) = attached("input");
        session
            .start(&request("/bin/cat", &[]))
            .expect("cat should start");

        session
            .input(b"typed-line\n")
            .expect("cat should take input");

        recorder.wait_for(b"typed-line");
    }

    /// `stty size` reads the window off the terminal itself, so agreeing with it
    /// is the only proof that a resize reached the child rather than only the
    /// host.
    #[test]
    fn a_resize_is_visible_to_the_child()
    {
        let (session, recorder) = attached("resize");
        session
            .start(&request("/bin/sh", &["-c", "sleep 0.5; stty size"]))
            .expect("sh should start");

        session
            .resize(PtyWindow {
                cols: 100,
                rows: 30
            })
            .expect("a resize should land");

        recorder.wait_for(b"30 100");
    }

    /// The race the app actually has: the WebView measures its terminal while
    /// the shell is still laying out, which is before `start` has been called.
    /// The size is kept and the child is spawned at it, so its first frame is
    /// already the right shape.
    #[test]
    fn a_resize_before_the_first_start_is_the_size_the_child_is_spawned_at()
    {
        let (session, recorder) = attached("resize-early");

        session
            .resize(PtyWindow {
                cols: 112,
                rows: 42
            })
            .expect("a resize with nothing running is accepted");

        let started = session
            .start(&request("/bin/sh", &["-c", "stty size"]))
            .expect("sh should start");

        assert_eq!(
            started.window,
            PtyWindow {
                cols: 112,
                rows: 42
            }
        );
        recorder.wait_for(b"42 112");
    }

    /// The other end of the same race: the terminal is still on screen after the
    /// child has gone, and it still changes size.
    #[test]
    fn a_resize_after_the_child_exits_is_kept_rather_than_refused()
    {
        let (session, recorder) = attached("resize-late");
        session
            .start(&request("/bin/echo", &["bye"]))
            .expect("echo should start");
        recorder.wait_for_state(PtyLifecycle::Exited);

        session
            .resize(PtyWindow { cols: 90, rows: 20 })
            .expect("a resize after the exit is accepted");

        assert_eq!(session.window(), PtyWindow { cols: 90, rows: 20 });
    }

    #[test]
    fn the_exit_reaches_the_sink()
    {
        let (session, recorder) = attached("exit");
        session
            .start(&request("/bin/echo", &["bye"]))
            .expect("echo should start");

        recorder.wait_for_state(PtyLifecycle::Exited);

        assert_eq!(session.lifecycle(), PtyLifecycle::Exited);
        assert_eq!(session.pid(), None);
    }

    /// Starting, then whatever the run reached, then gone — in that order and
    /// with nothing invented in between.
    ///
    /// The exact list is deliberately not asserted. A child that prints and
    /// exits inside a millisecond can be finished before the thread that spawned
    /// it has written `ready` down, so `ready` is allowed to be missing. What is
    /// not allowed is a state out of order, or a state twice.
    #[test]
    fn the_states_a_live_run_produces_move_forward_and_never_back()
    {
        let (session, recorder) = attached("states");
        session
            .start(&request("/bin/echo", &["hello"]))
            .expect("echo should start");

        recorder.wait_for_state(PtyLifecycle::Running);
        recorder.wait_for_state(PtyLifecycle::Exited);

        let states = recorder.states();
        let (attached_at, run) = states.split_at(1);

        assert_eq!(
            attached_at,
            [PtyLifecycle::Exited],
            "what `attach` reported"
        );
        assert_eq!(run.first(), Some(&PtyLifecycle::Starting));
        assert_eq!(run.last(), Some(&PtyLifecycle::Exited));

        for pair in run.windows(2)
        {
            assert!(pair[0].step() < pair[1].step(), "{run:?} went backwards");
        }
    }

    /// `running` says that output arrived, never what it was. Two children
    /// printing entirely different bytes reach the same state, which is what
    /// makes this a fact about the stream's existence rather than about its
    /// content.
    #[test]
    fn the_running_state_does_not_depend_on_what_the_child_printed()
    {
        for script in [
            "printf 'plain text'",
            "printf '\\033[2J\\033[H'",
            "printf '\\377\\376'"
        ]
        {
            let (session, recorder) = attached("running");
            session
                .start(&request("/bin/sh", &["-c", script]))
                .expect("sh should start");

            recorder.wait_for_state(PtyLifecycle::Running);
        }
    }

    /// A restart replaces the child, on real processes.
    #[test]
    fn a_restart_replaces_the_child()
    {
        let (session, recorder) = attached("restart");
        session
            .start(&request("/bin/sh", &["-c", "printf 'old '; sleep 30"]))
            .expect("sh should start");
        recorder.wait_for(b"old");

        let first = session.pid().expect("a first pid");
        session
            .restart(&request("/bin/cat", &[]))
            .expect("cat should start");
        let second = session.pid().expect("a second pid");

        assert_ne!(first, second);
        assert!(recorder.states().contains(&PtyLifecycle::Reconnecting));

        session.input(b"new\n").expect("cat should take input");
        recorder.wait_for(b"new");
    }

    /// The two things a restart must do that a real process cannot be asked
    /// about reliably: reap the child it replaces, and refuse what that child is
    /// still writing.
    ///
    /// A fake host is what makes both observable. `reap` is recorded rather than
    /// inferred from a process table, and the replaced child writes on demand
    /// rather than whenever the scheduler gets to it.
    #[test]
    fn a_restart_reaps_the_old_child_and_drops_what_it_was_still_writing()
    {
        let host = Arc::new(RecordingHost::default());
        let session = TerminalSession::new(Arc::clone(&host) as Arc<dyn PtyHost>, scratch("reap"));
        let recorder = Arc::new(Recorder::default());
        session.attach(Arc::clone(&recorder) as Arc<dyn TerminalSink>);

        session
            .start(&request("agent", &[]))
            .expect("the fake host always opens");
        let first = host.stream(0);
        first.wrote(b"before ");

        session
            .restart(&request("agent", &[]))
            .expect("the fake host always opens");

        assert_eq!(host.reaped(), 1, "the replaced child was not reaped");

        // The replaced child's reader is still running, as it would be for the
        // moment between the kill and the last read returning.
        first.wrote(b"after");
        first.ended();
        host.stream(1).wrote(b"fresh");

        assert_eq!(recorder.seen(), b"before fresh".to_vec());
        assert_ne!(
            session.lifecycle(),
            PtyLifecycle::Exited,
            "the replaced child's exit ended the new session"
        );
    }

    /// The scrollback is the session's, not the surface's, so a surface that
    /// went away and came back is shown what it missed.
    #[test]
    fn a_new_surface_is_replayed_what_the_previous_one_saw()
    {
        let scrollback = scratch("replay");
        let session = TerminalSession::new(Arc::new(SystemPtyHost), scrollback.clone());
        let first = Arc::new(Recorder::default());
        session.attach(Arc::clone(&first) as Arc<dyn TerminalSink>);

        session
            .start(&request("/bin/echo", &["persisted"]))
            .expect("echo should start");
        first.wait_for(b"persisted");

        // A second session over the same scrollback is what an app restart is.
        let restarted = TerminalSession::new(Arc::new(SystemPtyHost), scrollback);
        let second = Arc::new(Recorder::default());
        restarted.attach(Arc::clone(&second) as Arc<dyn TerminalSink>);

        assert!(
            second.holds(b"persisted"),
            "the replay lost the previous run"
        );
        assert_eq!(second.states(), vec![PtyLifecycle::Exited]);
    }

    /// A slow surface stops the reader rather than losing bytes. Every byte the
    /// child wrote arrives, in order, however long each hand-over took.
    #[test]
    fn a_slow_sink_receives_every_byte_in_order()
    {
        let session = session("backpressure");
        let recorder = Arc::new(Recorder::slow(Duration::from_millis(2)));
        session.attach(Arc::clone(&recorder) as Arc<dyn TerminalSink>);

        session
            .start(&request(
                "/bin/sh",
                &["-c", "for i in $(seq 1 300); do printf '%s.' $i; done"]
            ))
            .expect("sh should start");
        recorder.wait_for(b"300.");

        let seen = String::from_utf8(recorder.seen()).expect("digits and dots");
        let numbers: Vec<&str> = seen.trim_end_matches('.').split('.').collect();

        assert_eq!(numbers.len(), 300);
        assert_eq!(numbers.first(), Some(&"1"));
        assert_eq!(numbers.last(), Some(&"300"));
    }

    #[test]
    fn a_program_that_is_not_there_is_named_rather_than_spawned()
    {
        let (session, recorder) = attached("missing");

        assert_eq!(
            session.start(&request("trdr-no-such-agent-9c1f", &[])),
            Err(TerminalError::ExecutableMissing {
                name: "trdr-no-such-agent-9c1f".to_owned()
            })
        );
        assert_eq!(session.lifecycle(), PtyLifecycle::Exited);
        assert_eq!(
            recorder.states(),
            vec![
                PtyLifecycle::Exited,
                PtyLifecycle::Starting,
                PtyLifecycle::Exited
            ]
        );
    }

    #[test]
    fn speaking_to_a_session_that_is_not_running_is_an_error_rather_than_a_panic()
    {
        let session = session("idle");

        assert_eq!(session.input(b"x"), Err(TerminalError::NotRunning));
        assert_eq!(session.lifecycle(), PtyLifecycle::Exited);
        assert!(session.resize(PtyWindow { cols: 80, rows: 24 }).is_ok());
        assert!(session.pid().is_none());
    }

    /// Starting an already-running terminal answers with the one that is running
    /// rather than spawning a second child onto the same rail.
    #[test]
    fn starting_twice_does_not_produce_a_second_child()
    {
        let (session, _) = attached("twice");
        let first = session
            .start(&request("/bin/cat", &[]))
            .expect("cat should start");
        let again = session
            .start(&request("/bin/cat", &[]))
            .expect("the running one");

        assert_eq!(first.pid, again.pid);
    }
}
