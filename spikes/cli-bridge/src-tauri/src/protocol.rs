use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROTOCOL_VERSION: u32 = 1;

// `deny_unknown_fields` is the whole point of declaring these as structs rather
// than reading a Value: section 9.2 requires an unknown field to be refused, and
// a permissive parser silently accepts the field that a future protocol version
// will give a meaning to.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request
{
    pub v: u32,
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: Value
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response
{
    pub v: u32,
    pub id: Option<String>,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError
{
    pub code: String,
    pub message: String
}

impl Response
{
    pub fn ok(id: &str, result: Value) -> Self
    {
        Response { v: PROTOCOL_VERSION, id: Some(id.to_string()), ok: true, result: Some(result), error: None }
    }

    pub fn failed(id: Option<String>, code: &str, message: impl Into<String>) -> Self
    {
        let error = ResponseError { code: code.to_string(), message: message.into() };
        Response { v: PROTOCOL_VERSION, id, ok: false, result: None, error: Some(error) }
    }

    pub fn line(&self) -> String
    {
        // A response that cannot be serialized is a bug in this file, not a
        // runtime condition, so it must not be able to take the socket down.
        let mut text = serde_json::to_string(self).unwrap_or_else(|_| BROKEN_RESPONSE.to_string());
        text.push('\n');
        text
    }

    pub fn code(&self) -> Option<&str>
    {
        self.error.as_ref().map(|error| error.code.as_str())
    }
}

const BROKEN_RESPONSE: &str = r#"{"v":1,"id":null,"ok":false,"error":{"code":"INTERNAL","message":"response could not be encoded"}}"#;

#[derive(Debug, PartialEq, Eq)]
pub enum Method
{
    AppStatus,
    UiOpen,
    StrategyRegisterRequest,
    // Spike-only. Section 9.3 refuses an approval whose spec or data hash moved
    // while the sheet was up, and that path cannot be exercised without a way to
    // move the hash mid-request. It is not part of the contract in section 9.2
    // and does not carry into M2.
    SpikeSetHashes,
    // Spike-only, for the same reason: approve and reject are the two paths this
    // spike exists to demonstrate, and a physical click cannot be scripted
    // without an accessibility grant this checkout does not have.
    SpikeClick
}

impl Method
{
    pub fn parse(name: &str) -> Option<Method>
    {
        match name
        {
            "app.status" => Some(Method::AppStatus),
            "ui.open" => Some(Method::UiOpen),
            "strategy.register.request" => Some(Method::StrategyRegisterRequest),
            "spike.set_hashes" => Some(Method::SpikeSetHashes),
            "spike.click" => Some(Method::SpikeClick),
            _ => None
        }
    }
}

/// Reads one request line, or the response that should be written back instead.
///
/// A malformed line is still asked for its `id` before being refused. A client
/// matches responses to requests by id, so an error that drops the id leaves a
/// waiting caller with no way to tell which request failed.
pub fn read_request(line: &str) -> Result<Request, Response>
{
    let value: Value = serde_json::from_str(line)
        .map_err(|error| Response::failed(None, "BAD_REQUEST", format!("line is not JSON: {error}")))?;
    let id = value.get("id").and_then(Value::as_str).map(str::to_string);

    match value.get("v").and_then(Value::as_u64)
    {
        Some(version) if version == u64::from(PROTOCOL_VERSION) => {}
        Some(version) => return Err(Response::failed(id, "UNSUPPORTED_VERSION", format!("protocol v{version} is not supported"))),
        None => return Err(Response::failed(id, "BAD_REQUEST", "`v` is required and must be a number"))
    }

    serde_json::from_value(value)
        .map_err(|error| Response::failed(id, "BAD_REQUEST", error.to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterParams
{
    pub strategy_id: String,
    pub command: String,
    pub spec_hash: String,
    pub data_hash: String,
    #[serde(default = "default_expiry_seconds")]
    pub expires_in_seconds: u64
}

fn default_expiry_seconds() -> u64
{
    60
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetHashesParams
{
    pub strategy_id: String,
    pub spec_hash: String,
    pub data_hash: String
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClickParams
{
    pub approve: bool
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn refusal(line: &str) -> Response
    {
        read_request(line).expect_err("this line should have been refused")
    }

    #[test]
    fn a_well_formed_request_carries_its_method_and_params()
    {
        let line = r#"{"v":1,"id":"r1","method":"ui.open","params":{"resource":"strategy"}}"#;
        let request = read_request(line).expect("this line is well formed");
        assert_eq!(request.id, "r1");
        assert_eq!(Method::parse(&request.method), Some(Method::UiOpen));
        assert_eq!(request.params["resource"], "strategy");
    }

    #[test]
    fn params_may_be_left_out_entirely()
    {
        let request = read_request(r#"{"v":1,"id":"r1","method":"app.status"}"#).expect("params are optional");
        assert!(request.params.is_null());
    }

    #[test]
    fn a_missing_version_or_id_is_refused()
    {
        assert_eq!(refusal(r#"{"id":"r1","method":"app.status"}"#).code(), Some("BAD_REQUEST"));
        assert_eq!(refusal(r#"{"v":1,"method":"app.status"}"#).code(), Some("BAD_REQUEST"));
    }

    #[test]
    fn a_version_this_host_does_not_speak_is_refused_by_that_name()
    {
        let refused = refusal(r#"{"v":2,"id":"r1","method":"app.status"}"#);
        assert_eq!(refused.code(), Some("UNSUPPORTED_VERSION"));
    }

    // The field a later protocol version would give a meaning to. Accepting it
    // now means an old host silently ignores an instruction a new client meant.
    #[test]
    fn an_unknown_field_is_refused_rather_than_ignored()
    {
        let refused = refusal(r#"{"v":1,"id":"r1","method":"app.status","dry_run":true}"#);
        assert_eq!(refused.code(), Some("BAD_REQUEST"));
    }

    #[test]
    fn an_unknown_method_has_no_parse()
    {
        assert!(Method::parse("registration.commit").is_none());
        assert!(Method::parse("").is_none());
    }

    // A caller matches responses to requests by id. An error that drops the id
    // leaves it unable to tell which of its requests failed.
    #[test]
    fn a_refusal_still_answers_with_the_id_it_could_read()
    {
        assert_eq!(refusal(r#"{"v":9,"id":"r7","method":"app.status"}"#).id.as_deref(), Some("r7"));
        assert_eq!(refusal(r#"{"v":1,"id":"r7","method":"app.status","x":1}"#).id.as_deref(), Some("r7"));
    }

    #[test]
    fn a_line_that_is_not_json_is_refused_without_an_id()
    {
        let refused = refusal("not json at all");
        assert_eq!(refused.code(), Some("BAD_REQUEST"));
        assert!(refused.id.is_none());
    }

    #[test]
    fn every_response_is_one_line()
    {
        let line = Response::ok("r1", serde_json::json!({"status": "opened"})).line();
        assert!(line.ends_with('\n'));
        assert_eq!(line.matches('\n').count(), 1);
    }

    #[test]
    fn an_expiry_is_supplied_when_the_caller_leaves_it_out()
    {
        let params: RegisterParams = serde_json::from_value(serde_json::json!({
            "strategy_id": "low-vol-v1",
            "command": "trdr strategy register low-vol-v1",
            "spec_hash": "aaa",
            "data_hash": "bbb"
        }))
        .expect("expires_in_seconds is optional");
        assert_eq!(params.expires_in_seconds, 60);
    }
}
