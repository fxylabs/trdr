use std::time::Duration;

/// Section 13 names `HttpTransport` as a seam. It is also the boundary where the
/// spike's main risk is contained, which is why the error type below is as thin
/// as it is.
pub trait HttpTransport: Send + Sync
{
    fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<HttpResponse, TransportError>;
    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse, TransportError>;
}

pub struct HttpResponse
{
    pub status: u16,
    pub body: String
}

/// Deliberately carries nothing.
///
/// This is the leak the plan points at: an HTTP client's own error type prints
/// the request it failed on, and a request to KIS carries the app key, the app
/// secret and a bearer token in its headers. `{:?}` on such an error, or a
/// `map_err(|e| e.to_string())` on the way to a log, publishes all three. So the
/// client's error is read for its kind and then dropped here, at the one place
/// that ever sees it, rather than being carried inward as a cause chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError
{
    Timeout,
    Unavailable
}

const TIMEOUT: Duration = Duration::from_secs(3);

pub struct UreqTransport;

impl UreqTransport
{
    fn agent() -> ureq::Agent
    {
        ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            // Off on purpose. Left on, the client turns every 4xx and 5xx into
            // its own error type, and this file would then have to map "some
            // error" onto a code family — which collapses "your app key is
            // wrong" and "the broker is down" into one answer. A user reading
            // the second retries for ever on the first. The status has to
            // survive to the caller, so a non-2xx stays a response here.
            .http_status_as_error(false)
            .build()
            .into()
    }

    /// The one place a client error is looked at. Note what is not here: no
    /// `error.to_string()`, no `format!("{error:?}")`, nothing carried out.
    fn classify<T>(result: Result<T, ureq::Error>) -> Result<T, TransportError>
    {
        result.map_err(|error| match error
        {
            ureq::Error::Timeout(_) => TransportError::Timeout,
            _ => TransportError::Unavailable
        })
    }

    /// A non-2xx answer is a response, not a transport failure: the status is
    /// what the caller maps to `AUTH_*`, so it has to survive.
    fn read(response: ureq::http::Response<ureq::Body>) -> Result<HttpResponse, TransportError>
    {
        let status = response.status().as_u16();
        let body = response.into_body().read_to_string().map_err(|_| TransportError::Unavailable)?;
        Ok(HttpResponse { status, body })
    }
}

impl HttpTransport for UreqTransport
{
    fn post_json(&self, url: &str, headers: &[(&str, &str)], body: &str) -> Result<HttpResponse, TransportError>
    {
        let mut request = Self::agent().post(url).header("content-type", "application/json");
        for (name, value) in headers
        {
            request = request.header(*name, *value);
        }
        Self::read(Self::classify(request.send(body))?)
    }

    fn get(&self, url: &str, headers: &[(&str, &str)]) -> Result<HttpResponse, TransportError>
    {
        let mut request = Self::agent().get(url);
        for (name, value) in headers
        {
            request = request.header(*name, *value);
        }
        Self::read(Self::classify(request.call())?)
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    // The property the whole boundary exists for: whatever the client knows
    // about the failed request, none of it survives into our error.
    #[test]
    fn a_transport_error_prints_nothing_about_the_request()
    {
        for error in [TransportError::Timeout, TransportError::Unavailable]
        {
            let printed = format!("{error:?}");
            assert!(printed.len() < 20, "the error carries detail it should not: {printed}");
            for forbidden in ["http", "://", "authorization", "appkey", "Bearer"]
            {
                assert!(!printed.to_lowercase().contains(forbidden), "{printed} mentions {forbidden}");
            }
        }
    }
}
