use serde::{Deserialize, Serialize};

use crate::http::{HttpResponse, HttpTransport, TransportError};
use crate::mock::{BALANCE_PATH, TOKEN_PATH};
use crate::secret::Secret;

/// Section 12's stable families, narrowed to what this path can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KisError
{
    AuthMissing,
    AuthInvalid,
    AuthRateLimited,
    UpstreamUnavailable,
    UpstreamTimeout,
    UpstreamMalformed
}

impl KisError
{
    pub fn code(self) -> &'static str
    {
        match self
        {
            KisError::AuthMissing => "AUTH_MISSING",
            KisError::AuthInvalid => "AUTH_INVALID",
            KisError::AuthRateLimited => "AUTH_RATE_LIMITED",
            KisError::UpstreamUnavailable => "UPSTREAM_UNAVAILABLE",
            KisError::UpstreamTimeout => "UPSTREAM_TIMEOUT",
            KisError::UpstreamMalformed => "UPSTREAM_MALFORMED"
        }
    }
}

impl From<TransportError> for KisError
{
    fn from(error: TransportError) -> Self
    {
        match error
        {
            TransportError::Timeout => KisError::UpstreamTimeout,
            TransportError::Unavailable => KisError::UpstreamUnavailable
        }
    }
}

/// The credentials a KIS call needs. All three are secrets: the account number
/// is one too, because section 12 forbids it as a log field and section 5.2
/// forbids storing its raw form.
pub struct Credentials
{
    pub app_key: Secret,
    pub app_secret: Secret,
    pub account_no: Secret
}

/// `Debug` is derived on purpose rather than avoided. A bearer token ends up in
/// a `Result` that something calls `expect` on sooner or later, and that needs
/// `Debug`; the point of [`Secret`] is that deriving it stays safe.
#[derive(Debug)]
pub struct Token
{
    value: Secret
}

impl Token
{
    fn bearer(&self) -> String
    {
        format!("Bearer {}", self.value.expose())
    }
}

/// What is kept. No account number, no token, no upstream field names — section
/// 5.2 allows the normalized latest snapshot and nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountSnapshot
{
    pub total_evaluated: i64,
    pub cash: i64,
    pub holdings: Vec<Holding>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Holding
{
    pub symbol: String,
    pub name: String,
    pub quantity: i64,
    pub evaluated: i64
}

pub struct KisClient<T: HttpTransport>
{
    base_url: String,
    transport: T
}

impl<T: HttpTransport> KisClient<T>
{
    pub fn new(base_url: impl Into<String>, transport: T) -> Self
    {
        KisClient { base_url: base_url.into(), transport }
    }

    pub fn issue_token(&self, credentials: &Credentials) -> Result<Token, KisError>
    {
        if credentials.app_key.is_empty() || credentials.app_secret.is_empty()
        {
            return Err(KisError::AuthMissing);
        }

        // The body is built here rather than derived from a struct, so that the
        // secrets are exposed at one visible line instead of being carried into
        // a serializable type that something else might later print.
        let body = serde_json::json!({
            "grant_type": "client_credentials",
            "appkey": credentials.app_key.expose(),
            "appsecret": credentials.app_secret.expose()
        })
        .to_string();

        let response = self.transport.post_json(&format!("{}{TOKEN_PATH}", self.base_url), &[], &body)?;
        let body = read_ok(&response)?;

        let value = body
            .get("access_token")
            .and_then(|value| value.as_str())
            .ok_or(KisError::UpstreamMalformed)?;
        Ok(Token { value: Secret::new(value) })
    }

    /// Fetches the balance, reissuing once if the token has expired.
    ///
    /// The refresh is the path the plan warns about, so it is written to be the
    /// same code as the first issue rather than a second copy that could drift
    /// into logging what the first one does not.
    pub fn fetch_balance(&self, credentials: &Credentials, token: Token) -> Result<(AccountSnapshot, Token), KisError>
    {
        match self.balance_once(credentials, &token)
        {
            Ok(snapshot) => Ok((snapshot, token)),
            Err(KisError::AuthInvalid) =>
            {
                let fresh = self.issue_token(credentials)?;
                let snapshot = self.balance_once(credentials, &fresh)?;
                Ok((snapshot, fresh))
            }
            Err(other) => Err(other)
        }
    }

    fn balance_once(&self, credentials: &Credentials, token: &Token) -> Result<AccountSnapshot, KisError>
    {
        let response = self.transport.get(
            &format!("{}{BALANCE_PATH}", self.base_url),
            &[
                ("authorization", token.bearer().as_str()),
                ("appkey", credentials.app_key.expose()),
                ("appsecret", credentials.app_secret.expose()),
                ("tr_id", "TTTC8434R"),
                ("x-spike-account", credentials.account_no.expose())
            ]
        )?;
        normalize(&read_ok(&response)?)
    }
}

/// Maps a status to a code family. The body is never read for its message: an
/// upstream message is free-form text that has quoted a request back before now.
fn read_ok(response: &HttpResponse) -> Result<serde_json::Value, KisError>
{
    match response.status
    {
        200..=299 => serde_json::from_str(&response.body).map_err(|_| KisError::UpstreamMalformed),
        401 | 403 => Err(KisError::AuthInvalid),
        429 => Err(KisError::AuthRateLimited),
        _ => Err(KisError::UpstreamUnavailable)
    }
}

/// Keeps the four numbers a screen needs and drops the rest of the reply,
/// including the account number KIS echoes into every response.
fn normalize(body: &serde_json::Value) -> Result<AccountSnapshot, KisError>
{
    let totals = body.get("output2").and_then(|value| value.get(0)).ok_or(KisError::UpstreamMalformed)?;
    let holdings = body
        .get("output1")
        .and_then(|value| value.as_array())
        .ok_or(KisError::UpstreamMalformed)?
        .iter()
        .map(|row| Holding
        {
            symbol: text(row, "pdno"),
            name: text(row, "prdt_name"),
            quantity: number(row, "hldg_qty"),
            evaluated: number(row, "evlu_amt")
        })
        .collect();

    Ok(AccountSnapshot
    {
        total_evaluated: number(totals, "tot_evlu_amt"),
        cash: number(totals, "dnca_tot_amt"),
        holdings
    })
}

fn text(row: &serde_json::Value, field: &str) -> String
{
    row.get(field).and_then(|value| value.as_str()).unwrap_or_default().to_string()
}

fn number(row: &serde_json::Value, field: &str) -> i64
{
    row.get(field).and_then(|value| value.as_str()).and_then(|value| value.parse().ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::http::UreqTransport;
    use crate::mock::{MockKis, Mode};

    const CANARY: &str = "TRDR-CANARY-6f2a9c";
    const ACCOUNT: &str = "TRDR-ACCOUNT-50112233";

    fn credentials() -> Credentials
    {
        Credentials
        {
            app_key: Secret::new("spike-app-key"),
            app_secret: Secret::new(CANARY),
            account_no: Secret::new(ACCOUNT)
        }
    }

    fn client(mock: &MockKis) -> KisClient<UreqTransport>
    {
        KisClient::new(mock.base_url(), UreqTransport)
    }

    fn mock() -> MockKis
    {
        MockKis::start("spike-app-key", CANARY).expect("the mock could not start")
    }

    #[test]
    fn an_authenticated_request_returns_a_normalised_snapshot()
    {
        let mock = mock();
        let client = client(&mock);
        let token = client.issue_token(&credentials()).expect("a token should be issued");
        let (snapshot, _) = client.fetch_balance(&credentials(), token).expect("the balance should read");

        assert_eq!(snapshot.total_evaluated, 2_560_000);
        assert_eq!(snapshot.cash, 1_000_000);
        assert_eq!(snapshot.holdings.len(), 2);
        assert_eq!(snapshot.holdings[0].symbol, "005930");
        assert_eq!(snapshot.holdings[0].quantity, 12);
    }

    // The account number reaches KIS on every call and comes back in every
    // reply. What is kept must not contain it.
    #[test]
    fn the_snapshot_keeps_no_account_number_and_no_token()
    {
        let mock = mock();
        let client = client(&mock);
        let token = client.issue_token(&credentials()).expect("a token should be issued");
        let (snapshot, _) = client.fetch_balance(&credentials(), token).expect("the balance should read");

        let json = serde_json::to_string(&snapshot).expect("a snapshot should serialise");
        assert!(!json.contains(ACCOUNT), "the account number survived normalisation");
        assert!(!json.contains(CANARY), "the app secret survived normalisation");
        assert!(!json.contains("mock-access-token"), "the token survived normalisation");
        assert!(!json.contains("cano"), "an upstream field name survived normalisation");
    }

    // The refresh path the plan calls out. The token count is what proves a
    // second token was really issued rather than the old one retried.
    #[test]
    fn an_expired_token_is_reissued_once_and_the_call_succeeds()
    {
        let mock = mock();
        let client = client(&mock);
        let token = client.issue_token(&credentials()).expect("a token should be issued");
        assert_eq!(mock.tokens_issued(), 1);

        mock.set_mode(Mode::TokenExpiredOnce);
        let (snapshot, _) = client.fetch_balance(&credentials(), token).expect("the balance should read after a refresh");
        assert_eq!(mock.tokens_issued(), 2, "the client retried without reissuing");
        assert_eq!(snapshot.total_evaluated, 2_560_000);
    }

    #[test]
    fn a_token_prints_nothing_of_itself()
    {
        let mock = mock();
        let token = client(&mock).issue_token(&credentials()).expect("a token should be issued");
        let printed = format!("{token:?}");
        assert!(!printed.contains("mock-access-token"), "the token printed itself: {printed}");
    }

    #[test]
    fn a_wrong_secret_is_auth_invalid_and_says_nothing_else()
    {
        let mock = mock();
        let wrong = Credentials
        {
            app_key: Secret::new("spike-app-key"),
            app_secret: Secret::new("TRDR-CANARY-wrong"),
            account_no: Secret::new(ACCOUNT)
        };
        let error = client(&mock).issue_token(&wrong).expect_err("a wrong secret should be refused");
        assert_eq!(error.code(), "AUTH_INVALID");
        assert!(!format!("{error:?}").contains("TRDR-CANARY"));
    }

    #[test]
    fn an_empty_credential_never_reaches_the_network()
    {
        let mock = mock();
        let empty = Credentials
        {
            app_key: Secret::new(""),
            app_secret: Secret::new(""),
            account_no: Secret::new(ACCOUNT)
        };
        assert_eq!(client(&mock).issue_token(&empty).expect_err("empty should be refused"), KisError::AuthMissing);
        assert_eq!(mock.tokens_issued(), 0);
    }

    #[test]
    fn every_upstream_failure_maps_to_a_code_and_carries_no_detail()
    {
        for (mode, expected) in [
            (Mode::RateLimited, "AUTH_RATE_LIMITED"),
            (Mode::Unavailable, "UPSTREAM_UNAVAILABLE"),
            (Mode::Malformed, "UPSTREAM_MALFORMED"),
            (Mode::Slow, "UPSTREAM_TIMEOUT")
        ]
        {
            let mock = mock();
            mock.set_mode(mode);
            let error = client(&mock).issue_token(&credentials()).expect_err("this mode should fail");
            assert_eq!(error.code(), expected, "{mode:?} mapped to the wrong code");

            let printed = format!("{error:?}");
            assert!(!printed.contains(CANARY), "{mode:?} printed the secret: {printed}");
            assert!(!printed.contains("http"), "{mode:?} printed the request: {printed}");
        }
    }
}
