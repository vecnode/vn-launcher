//! Plain, unauthenticated HTTPS.
//!
//! There is no account, no token and no login anywhere in this launcher: every
//! request goes to the public GitHub API as an anonymous guest. What that buys in
//! simplicity it costs in rate limit, which is why the caller caches aggressively
//! (see [`crate::cache`]) rather than relying on this module to be frugal.

use std::time::Duration;

use crate::error::{Error, Result};

/// The largest releases response the launcher will read.
pub const MAX_FEED_BYTES: u64 = 4 * 1024 * 1024;

/// The largest repository-probe response the launcher will read.
pub const MAX_PROBE_BYTES: u64 = 1024 * 1024;

/// How long to wait for a connection.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long to wait for an entire request and response.
const TOTAL_TIMEOUT: Duration = Duration::from_secs(25);

/// An HTTPS client that identifies the launcher honestly and verifies certificates
/// against the operating system's own trust store.
#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
}

impl Client {
    /// Build a client with the launcher's timeouts and user agent.
    #[must_use]
    pub fn new() -> Self {
        let user_agent = format!(
            "vn-launcher/{} (+https://github.com/vecnode/vn-launcher)",
            vnl_core::VERSION
        );

        // Root certificates are named explicitly, and this is not optional. Building
        // `ureq` without its default features leaves `RootCerts` at its own default,
        // `WebPki`, whose backing crate is deliberately not compiled in — and rather
        // than failing the request, that configuration *panics* on first use. Naming
        // the platform verifier is what makes the launcher use the operating
        // system's trust store, which is also the licensing-safe choice.
        let tls = ureq::tls::TlsConfig::builder()
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build();

        let config = ureq::Agent::config_builder()
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_global(Some(TOTAL_TIMEOUT))
            // A 404 from the releases endpoint means "this repository publishes no
            // releases", which is ordinary information, not an exception to throw.
            .http_status_as_error(false)
            .user_agent(user_agent)
            .tls_config(tls)
            .build();

        Self {
            agent: ureq::Agent::new_with_config(config),
        }
    }

    /// Fetch a URL and return its status and body.
    ///
    /// The body is read through a hard limit, so a hostile or broken server cannot
    /// exhaust the launcher's memory.
    ///
    /// # Errors
    /// Returns [`Error::Network`] when the request never completed and
    /// [`Error::TooLarge`] when the body exceeded `limit`. A non-2xx status is
    /// returned as ordinary data, not an error.
    pub fn get_json(&self, url: &str, limit: u64) -> Result<(u16, String)> {
        let mut response = self
            .agent
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .call()
            .map_err(|error| match error {
                ureq::Error::BodyExceedsLimit(_) => Error::TooLarge {
                    url: url.to_owned(),
                    limit,
                },
                other => Error::Network {
                    url: url.to_owned(),
                    reason: other.to_string(),
                },
            })?;

        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(limit)
            .read_to_string()
            .map_err(|error| match error {
                ureq::Error::BodyExceedsLimit(_) => Error::TooLarge {
                    url: url.to_owned(),
                    limit,
                },
                other => Error::Network {
                    url: url.to_owned(),
                    reason: other.to_string(),
                },
            })?;

        Ok((status, body))
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for Client {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Client")
    }
}

/// Pull GitHub's own `{"message": …}` out of an error body, when it is one.
#[must_use]
pub fn api_message(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    let message = value.get("message")?.as_str()?.trim();
    (!message.is_empty()).then(|| message.to_owned())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn reads_githubs_explanation_when_it_gives_one() {
        assert_eq!(
            api_message(r#"{"message":"API rate limit exceeded for 1.2.3.4."}"#).as_deref(),
            Some("API rate limit exceeded for 1.2.3.4.")
        );
    }

    #[test]
    fn says_nothing_when_there_is_no_explanation() {
        assert!(api_message("").is_none());
        assert!(api_message("[]").is_none());
        assert!(api_message(r#"{"message":"   "}"#).is_none());
        assert!(api_message("not json").is_none());
    }
}
