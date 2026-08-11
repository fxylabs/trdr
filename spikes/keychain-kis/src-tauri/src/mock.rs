//! A stand-in for KIS, on loopback.
//!
//! The plan forbids this spike from calling the real broker, and a mock is also
//! the only way to reach the failure paths on purpose: an expired token, a rate
//! limit, a body that is not JSON. Those are where the plan says a secret
//! usually escapes, so they are the ones worth being able to run at will.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub const TOKEN_PATH: &str = "/oauth2/tokenP";
pub const BALANCE_PATH: &str = "/uapi/domestic-stock/v1/trading/inquire-balance";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode
{
    #[default]
    Healthy,
    /// The balance call refuses once with an expired token, so the client has to
    /// notice and reissue. This is the refresh path.
    TokenExpiredOnce,
    RateLimited,
    Unavailable,
    Malformed,
    /// Answers later than any sane client timeout.
    Slow
}

pub struct MockKis
{
    address: SocketAddr,
    state: Arc<State>
}

struct State
{
    app_key: String,
    app_secret: String,
    mode: Mutex<Mode>,
    issued: AtomicU64,
    refusals: AtomicU64
}

impl MockKis
{
    pub fn start(app_key: &str, app_secret: &str) -> std::io::Result<MockKis>
    {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let state = Arc::new(State
        {
            app_key: app_key.to_string(),
            app_secret: app_secret.to_string(),
            mode: Mutex::new(Mode::Healthy),
            issued: AtomicU64::new(0),
            refusals: AtomicU64::new(0)
        });

        let serving = Arc::clone(&state);
        thread::spawn(move ||
        {
            for connection in listener.incoming().flatten()
            {
                let state = Arc::clone(&serving);
                thread::spawn(move || answer(&state, connection));
            }
        });

        Ok(MockKis { address, state })
    }

    pub fn base_url(&self) -> String
    {
        format!("http://{}", self.address)
    }

    pub fn set_mode(&self, mode: Mode)
    {
        *self.state.mode.lock().unwrap() = mode;
        self.state.refusals.store(0, Ordering::SeqCst);
    }

    /// How many tokens this mock has handed out. A refresh that did not actually
    /// reissue would leave this at one, so the count is what tells a working
    /// refresh from a retry that silently reused the old token.
    pub fn tokens_issued(&self) -> u64
    {
        self.state.issued.load(Ordering::SeqCst)
    }
}

struct Request
{
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String
}

impl Request
{
    fn header(&self, name: &str) -> &str
    {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
            .unwrap_or_default()
    }
}

fn answer(state: &State, mut connection: TcpStream)
{
    let Some(request) = read_request(&mut connection) else { return };
    let mode = *state.mode.lock().unwrap();

    if mode == Mode::Slow
    {
        thread::sleep(Duration::from_secs(10));
    }

    let (status, body) = match (mode, request.method.as_str(), request.path.as_str())
    {
        (Mode::Unavailable, _, _) => (500, r#"{"msg1":"upstream is down"}"#.to_string()),
        (Mode::Malformed, _, _) => (200, "{ this is not json".to_string()),
        (Mode::RateLimited, _, _) => (429, r#"{"msg_cd":"EGW00201","msg1":"initial token issue exceeded"}"#.to_string()),
        (_, "POST", TOKEN_PATH) => issue_token(state, &request),
        (_, "GET", BALANCE_PATH) => balance(state, mode, &request),
        _ => (404, r#"{"msg1":"no such path"}"#.to_string())
    };

    let response = format!(
        "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = connection.write_all(response.as_bytes());
}

fn issue_token(state: &State, request: &Request) -> (u16, String)
{
    let sent: serde_json::Value = serde_json::from_str(&request.body).unwrap_or_default();
    let key = sent.get("appkey").and_then(|value| value.as_str()).unwrap_or_default();
    let secret = sent.get("appsecret").and_then(|value| value.as_str()).unwrap_or_default();

    if key != state.app_key || secret != state.app_secret
    {
        return (401, r#"{"msg_cd":"EGW00133","msg1":"appkey or appsecret is invalid"}"#.to_string());
    }

    let serial = state.issued.fetch_add(1, Ordering::SeqCst) + 1;
    (200, format!(r#"{{"access_token":"mock-access-token-{serial}","token_type":"Bearer","expires_in":86400}}"#))
}

fn balance(state: &State, mode: Mode, request: &Request) -> (u16, String)
{
    if !request.header("authorization").starts_with("Bearer mock-access-token-")
    {
        return (401, r#"{"msg_cd":"EGW00121","msg1":"token is not valid"}"#.to_string());
    }
    if mode == Mode::TokenExpiredOnce && state.refusals.fetch_add(1, Ordering::SeqCst) == 0
    {
        return (401, r#"{"msg_cd":"EGW00123","msg1":"token has expired"}"#.to_string());
    }

    // Shaped like the real answer, including the parts that must never be
    // persisted: the account number is echoed back in every KIS reply.
    let account = request.header("x-spike-account");
    (
        200,
        format!(
            r#"{{"rt_cd":"0","cano":"{account}","output1":[{{"pdno":"005930","prdt_name":"Samsung Electronics","hldg_qty":"12","evlu_amt":"948000"}},{{"pdno":"000660","prdt_name":"SK hynix","hldg_qty":"3","evlu_amt":"612000"}}],"output2":[{{"tot_evlu_amt":"2560000","dnca_tot_amt":"1000000"}}]}}"#
        )
    )
}

fn read_request(connection: &mut TcpStream) -> Option<Request>
{
    let mut reader = BufReader::new(connection.try_clone().ok()?);
    let mut start = String::new();
    reader.read_line(&mut start).ok()?;
    let mut words = start.split_whitespace();
    let method = words.next()?.to_string();
    let path = words.next()?.split('?').next()?.to_string();

    let mut headers = Vec::new();
    loop
    {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 || line.trim().is_empty()
        {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
        {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }

    let length: usize = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    if length > 0
    {
        reader.read_exact(&mut body).ok()?;
    }

    Some(Request { method, path, headers, body: String::from_utf8_lossy(&body).to_string() })
}
