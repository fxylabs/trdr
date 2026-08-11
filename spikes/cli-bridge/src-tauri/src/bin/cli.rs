//! The other end of the round trip.
//!
//! It is a real separate process, connecting over the real socket and blocking
//! on the real response, because that is the only arrangement that can answer
//! the question spike 3 exists for: does a terminal result reach the CLI process
//! that asked for it, whichever way the user answered.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use trdr_spike_cli_bridge_lib::protocol::{Response, PROTOCOL_VERSION};
use trdr_spike_cli_bridge_lib::runtime;

const USAGE: &str = "\
trdr-spike-cli — the CLI half of the M1 socket bridge spike

    status                                  ask the running app whether it is there
    open <resource> <id>                    raise and focus the app window
    register <strategy-id> [seconds]        request a registration approval and wait for it
    replay <request-id> <strategy-id>       send a request id that was already answered
    move-hashes <strategy-id> <spec-hash>   move a strategy's hashes while a sheet is up
    click approve|reject                    press a button on the sheet that is on screen

Options:
    --format json                           print the raw response instead of a sentence
";

fn main() -> ExitCode
{
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let json_output = arguments.iter().any(|argument| argument == "json")
        && arguments.iter().any(|argument| argument == "--format");
    let words: Vec<&str> = arguments.iter().map(String::as_str).filter(|word| *word != "--format" && *word != "json").collect();

    let request = match build_request(&words)
    {
        Some(request) => request,
        None =>
        {
            eprint!("{USAGE}");
            return ExitCode::from(64);
        }
    };

    match ask(&request)
    {
        Ok(response) => report(&response, json_output),
        Err(problem) =>
        {
            // Section 9.2: with no app running there is nothing to ask, and the
            // CLI says that rather than reporting a socket error at the user.
            eprintln!("APP_NOT_RUNNING — {problem}");
            ExitCode::from(69)
        }
    }
}

fn build_request(words: &[&str]) -> Option<Value>
{
    let method_and_params = match words
    {
        ["status"] => ("app.status", Value::Null, fresh_id()),
        ["open", resource, id] => ("ui.open", json!({"resource": resource, "id": id}), fresh_id()),
        ["register", strategy_id] => ("strategy.register.request", register_params(strategy_id, 60), fresh_id()),
        ["register", strategy_id, seconds] =>
        {
            ("strategy.register.request", register_params(strategy_id, seconds.parse().ok()?), fresh_id())
        }
        ["replay", request_id, strategy_id] =>
        {
            ("strategy.register.request", register_params(strategy_id, 60), (*request_id).to_string())
        }
        ["click", "approve"] => ("spike.click", json!({"approve": true}), fresh_id()),
        ["click", "reject"] => ("spike.click", json!({"approve": false}), fresh_id()),
        ["move-hashes", strategy_id, spec_hash] =>
        {
            ("spike.set_hashes", json!({"strategy_id": strategy_id, "spec_hash": spec_hash, "data_hash": "data-1"}), fresh_id())
        }
        _ => return None
    };

    let (method, params, id) = method_and_params;
    Some(json!({"v": PROTOCOL_VERSION, "id": id, "method": method, "params": params}))
}

fn register_params(strategy_id: &str, seconds: u64) -> Value
{
    json!({
        "strategy_id": strategy_id,
        "command": format!("trdr strategy register {strategy_id}"),
        "spec_hash": "spec-1",
        "data_hash": "data-1",
        "expires_in_seconds": seconds
    })
}

fn fresh_id() -> String
{
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|since| since.subsec_nanos()).unwrap_or_default();
    format!("{}-{nanos}", std::process::id())
}

fn ask(request: &Value) -> Result<Response, String>
{
    let socket = runtime::socket_path().map_err(|error| error.to_string())?;
    let stream = UnixStream::connect(&socket).map_err(|error| format!("{}: {error}", socket.display()))?;

    // Long enough that a user has time to read the sheet, and short enough that a
    // wedged app does not hold the terminal for ever. The app's own expiry is the
    // real bound; this is the backstop for an app that stopped answering.
    stream.set_read_timeout(Some(Duration::from_secs(600))).map_err(|error| error.to_string())?;

    let mut out = stream.try_clone().map_err(|error| error.to_string())?;
    out.write_all(format!("{request}\n").as_bytes()).map_err(|error| error.to_string())?;

    let mut line = String::new();
    BufReader::new(&stream).read_line(&mut line).map_err(|error| error.to_string())?;
    if line.trim().is_empty()
    {
        return Err("the app closed the connection without answering".to_string());
    }
    serde_json::from_str(&line).map_err(|error| format!("the response was not readable: {error}"))
}

fn report(response: &Response, json_output: bool) -> ExitCode
{
    if json_output
    {
        println!("{}", serde_json::to_string(response).unwrap_or_default());
    }
    else if response.ok
    {
        println!("ok  {}", response.result.as_ref().map(Value::to_string).unwrap_or_default());
    }
    else
    {
        let error = response.error.as_ref();
        println!(
            "{}  {}",
            error.map(|error| error.code.as_str()).unwrap_or("FAILED"),
            error.map(|error| error.message.as_str()).unwrap_or_default()
        );
    }

    // A rejected or expired approval is a refusal, and a shell that reads exit
    // codes has to be able to tell it from an approval.
    if response.ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
