use std::io::{Read, Write};
use std::sync::Mutex;

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};

use crate::env;

// The host owns the PTY and the WebView renders it. Output leaves through a
// sink the caller supplies, so nothing in this module knows what happens to the
// bytes — which is what keeps "the app must not parse the stream" a property of
// the code rather than a promise in a comment.
pub type OutputSink = Box<dyn Fn(&[u8]) + Send + 'static>;
pub type ClosedSink = Box<dyn FnOnce() + Send + 'static>;

pub struct Pty
{
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>
}

#[derive(Default)]
pub struct PtyState(pub Mutex<Option<Pty>>);

#[derive(Deserialize)]
pub struct SpawnRequest
{
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub cols: u16,
    pub rows: u16
}

#[derive(Serialize)]
pub struct SpawnInfo
{
    pub program: String,
    pub pid: Option<u32>
}

pub fn open(request: &SpawnRequest, output: OutputSink, closed: ClosedSink) -> Result<(Pty, SpawnInfo), String>
{
    let program = env::resolve_program(&request.program)
        .ok_or_else(|| format!("`{}` is not on the login shell's PATH — point at the executable instead", request.program))?;
    let pair = native_pty_system().openpty(size(request.cols, request.rows)).map_err(text)?;
    let child = pair.slave.spawn_command(command(&program, request)).map_err(text)?;
    drop(pair.slave);
    let reader = pair.master.try_clone_reader().map_err(text)?;
    let writer = pair.master.take_writer().map_err(text)?;
    let info = SpawnInfo { program, pid: child.process_id() };
    pump(reader, output, closed);
    Ok((Pty { master: pair.master, writer, child }, info))
}

// The child gets the login shell's PATH, not the app's. Without it a resolved
// `claude` starts and then cannot find the tools it shells out to, which looks
// like a broken agent rather than a broken environment.
fn command(program: &str, request: &SpawnRequest) -> CommandBuilder
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

fn size(cols: u16, rows: u16) -> PtySize
{
    PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }
}

fn text(error: impl std::fmt::Display) -> String
{
    error.to_string()
}

fn pump(mut reader: Box<dyn Read + Send>, output: OutputSink, closed: ClosedSink)
{
    std::thread::spawn(move ||
    {
        let mut buffer = [0u8; 8192];
        loop
        {
            match reader.read(&mut buffer)
            {
                Ok(0) => break,
                Ok(count) => output(&buffer[..count]),
                Err(_) => break
            }
        }
        closed();
    });
}

impl Pty
{
    pub fn write(&mut self, data: &str) -> Result<(), String>
    {
        self.writer.write_all(data.as_bytes()).map_err(text)?;
        self.writer.flush().map_err(text)
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), String>
    {
        self.master.resize(size(cols, rows)).map_err(text)
    }

    pub fn kill(&mut self) -> Result<(), String>
    {
        self.child.kill().map_err(text)
    }
}

pub fn hold(state: &PtyState, pty: Pty) -> Result<(), String>
{
    *state.0.lock().map_err(poisoned)? = Some(pty);
    Ok(())
}

pub fn write(state: &PtyState, data: &str) -> Result<(), String>
{
    state.0.lock().map_err(poisoned)?.as_mut().ok_or("no pty is running")?.write(data)
}

pub fn resize(state: &PtyState, cols: u16, rows: u16) -> Result<(), String>
{
    state.0.lock().map_err(poisoned)?.as_mut().ok_or("no pty is running")?.resize(cols, rows)
}

pub fn kill(state: &PtyState) -> Result<(), String>
{
    let mut held = state.0.lock().map_err(poisoned)?;
    if let Some(pty) = held.as_mut()
    {
        pty.kill()?;
    }
    *held = None;
    Ok(())
}

pub fn running(state: &PtyState) -> bool
{
    state.0.lock().map(|held| held.is_some()).unwrap_or(false)
}

fn poisoned<T>(_: T) -> String
{
    "pty state is poisoned".to_string()
}

#[cfg(test)]
mod tests
{
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::*;

    struct Recorder
    {
        bytes: Arc<Mutex<Vec<u8>>>,
        closed: Arc<Mutex<bool>>
    }

    impl Recorder
    {
        fn new() -> Self
        {
            Self { bytes: Arc::new(Mutex::new(Vec::new())), closed: Arc::new(Mutex::new(false)) }
        }

        fn sinks(&self) -> (OutputSink, ClosedSink)
        {
            let bytes = self.bytes.clone();
            let closed = self.closed.clone();
            (
                Box::new(move |chunk: &[u8]| bytes.lock().unwrap().extend_from_slice(chunk)),
                Box::new(move || *closed.lock().unwrap() = true)
            )
        }

        fn seen(&self) -> Vec<u8>
        {
            self.bytes.lock().unwrap().clone()
        }
    }

    // Absolute paths on purpose: what resolution does with a bare name is
    // env's question, and mixing it in here would make a PTY failure look
    // like a PATH failure.
    fn request(program: &str, args: &[&str], cols: u16, rows: u16) -> SpawnRequest
    {
        SpawnRequest
        {
            program: program.to_string(),
            args: args.iter().map(|argument| argument.to_string()).collect(),
            cwd: None,
            cols,
            rows
        }
    }

    fn wait_for(recorder: &Recorder, needle: &[u8]) -> Vec<u8>
    {
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline
        {
            let seen = recorder.seen();
            if seen.windows(needle.len()).any(|window| window == needle)
            {
                return seen;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("never saw {:?} — got {:?}", String::from_utf8_lossy(needle), String::from_utf8_lossy(&recorder.seen()));
    }

    #[test]
    fn a_child_process_output_reaches_the_sink()
    {
        let recorder = Recorder::new();
        let (sink, closed) = recorder.sinks();
        let (_pty, info) = open(&request("/bin/echo", &["from-the-pty"], 80, 24), sink, closed).unwrap();
        assert!(info.pid.is_some());
        wait_for(&recorder, b"from-the-pty");
    }

    // The point of the whole spike: escape sequences must arrive as the bytes
    // the child emitted, not as anything the host decided they meant.
    #[test]
    fn ansi_escapes_arrive_as_raw_bytes()
    {
        let recorder = Recorder::new();
        let (sink, closed) = recorder.sinks();
        let script = "printf '\\033[31mred\\033[0m\\n'";
        let (_pty, _) = open(&request("/bin/sh", &["-c", script], 80, 24), sink, closed).unwrap();
        let seen = wait_for(&recorder, b"red");
        let escape: &[u8] = &[0x1b, b'[', b'3', b'1', b'm'];
        assert!(seen.windows(escape.len()).any(|window| window == escape), "the CSI sequence was rewritten or stripped");
    }

    #[test]
    fn typed_input_reaches_the_child()
    {
        let recorder = Recorder::new();
        let (sink, closed) = recorder.sinks();
        let (mut pty, _) = open(&request("/bin/cat", &[], 80, 24), sink, closed).unwrap();
        pty.write("typed-line\n").unwrap();
        wait_for(&recorder, b"typed-line");
        pty.kill().unwrap();
    }

    // `stty size` reads the window from the tty itself, so agreeing with it is
    // the only proof that a resize reached the child rather than only the host.
    #[test]
    fn a_resize_is_visible_to_the_child()
    {
        let recorder = Recorder::new();
        let (sink, closed) = recorder.sinks();
        let script = "sleep 0.5; stty size";
        let (pty, _) = open(&request("/bin/sh", &["-c", script], 80, 24), sink, closed).unwrap();
        pty.resize(100, 30).unwrap();
        wait_for(&recorder, b"30 100");
    }

    #[test]
    fn the_closed_sink_fires_when_the_child_exits()
    {
        let recorder = Recorder::new();
        let (sink, closed) = recorder.sinks();
        let (_pty, _) = open(&request("/bin/echo", &["bye"], 80, 24), sink, closed).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline && !*recorder.closed.lock().unwrap()
        {
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(*recorder.closed.lock().unwrap(), "the exit never reached the sink");
    }

    #[test]
    fn writing_with_no_session_is_an_error_rather_than_a_panic()
    {
        let state = PtyState::default();
        assert!(write(&state, "x").is_err());
        assert!(resize(&state, 80, 24).is_err());
        assert!(!running(&state));
        assert!(kill(&state).is_ok());
    }
}
