//! HTTP access to a Hue Bridge's local API (CLIP v2).

use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use serde_json::{Value, json};

use crate::home::Request;
use crate::pinning::{Fingerprint, PinnedVerifier};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("the link button on the bridge has not been pressed")]
    LinkButtonNotPressed,
    #[error("the bridge rejected the application key; pair again")]
    Unauthorized,
    #[error("bridge error: {0}")]
    Bridge(String),
    #[error("the bridge presented a different certificate than when it was paired")]
    CertificateMismatch,
}

pub type Result<T> = std::result::Result<T, Error>;

/// Credentials received when pairing.
#[derive(Debug, Clone)]
pub struct Pairing {
    pub app_key: String,
    pub client_key: Option<String>,
    /// The certificate the bridge presented while pairing.
    pub certificate: Fingerprint,
}

/// A client whose TLS connections are checked by `verifier` (see
/// [`crate::pinning`] for why the usual CA verification can't be used).
fn http_client(verifier: &Arc<PinnedVerifier>, timeout: Option<Duration>) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .tls_backend_preconfigured(verifier.client_config())
        .connect_timeout(Duration::from_secs(5))
        // The event stream can be quiet for hours. Keepalive probes notice a
        // bridge that vanished without closing the connection (power or Wi-Fi
        // loss) within about a minute, so the stream gets reopened.
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_keepalive_interval(Duration::from_secs(10))
        .tcp_keepalive_retries(3u32);
    if let Some(t) = timeout {
        builder = builder.timeout(t);
    }
    builder.build().expect("TLS backend is available")
}

/// Basic bridge information that needs no authentication.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BridgeInfo {
    pub id: String,
    pub name: String,
    pub address: String,
    pub model: String,
}

/// Fetches the bridge's public information and the fingerprint of the
/// certificate it presented. Sends nothing secret, so it's safe before the
/// certificate is known.
pub async fn bridge_info_with_certificate(address: &str) -> Result<(BridgeInfo, Fingerprint)> {
    let verifier = PinnedVerifier::recording();
    let v: Value = http_client(&verifier, Some(Duration::from_secs(5)))
        .get(format!("https://{address}/api/0/config"))
        .send()
        .await?
        .json()
        .await?;
    let certificate = verifier
        .seen()
        .ok_or_else(|| Error::Bridge("no certificate seen".into()))?;
    let s = |k: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let info = BridgeInfo {
        id: s("bridgeid").to_lowercase(),
        name: s("name"),
        address: address.to_owned(),
        model: s("modelid"),
    };
    Ok((info, certificate))
}

pub async fn bridge_info(address: &str) -> Result<BridgeInfo> {
    Ok(bridge_info_with_certificate(address).await?.0)
}

/// Asks the bridge for an application key. Fails with
/// [`Error::LinkButtonNotPressed`] until the user presses the button.
/// The key is bound to the certificate seen on this same connection.
pub async fn pair(address: &str, device_name: &str) -> Result<Pairing> {
    let verifier = PinnedVerifier::recording();
    let v: Value = http_client(&verifier, Some(Duration::from_secs(10)))
        .post(format!("https://{address}/api"))
        .json(&json!({"devicetype": format!("kandela#{device_name}"), "generateclientkey": true}))
        .send()
        .await?
        .json()
        .await?;
    let first = v.get(0).cloned().unwrap_or(Value::Null);
    if let Some(success) = first.get("success") {
        let key = |k: &str| success.get(k).and_then(Value::as_str).map(str::to_owned);
        return Ok(Pairing {
            app_key: key("username").ok_or_else(|| Error::Bridge("no username in reply".into()))?,
            client_key: key("clientkey"),
            certificate: verifier
                .seen()
                .ok_or_else(|| Error::Bridge("no certificate seen".into()))?,
        });
    }
    match first.pointer("/error/type").and_then(Value::as_i64) {
        Some(101) => Err(Error::LinkButtonNotPressed),
        _ => Err(Error::Bridge(
            first
                .pointer("/error/description")
                .and_then(Value::as_str)
                .unwrap_or("unexpected reply")
                .to_owned(),
        )),
    }
}

/// An authenticated connection to one bridge. Only talks to the bridge
/// whose certificate matches `certificate`.
#[derive(Clone)]
pub struct Bridge {
    address: String,
    app_key: String,
    verifier: Arc<PinnedVerifier>,
    http: reqwest::Client,
    stream_http: reqwest::Client,
}

impl Bridge {
    pub fn new(address: &str, app_key: &str, certificate: Fingerprint) -> Self {
        let verifier = PinnedVerifier::pinned(certificate);
        Self {
            address: address.to_owned(),
            app_key: app_key.to_owned(),
            http: http_client(&verifier, Some(Duration::from_secs(10))),
            // The event stream is a long-lived response; no overall timeout.
            stream_http: http_client(&verifier, None),
            verifier,
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    fn connection_error(&self, e: reqwest::Error) -> Error {
        if self.verifier.mismatched() {
            Error::CertificateMismatch
        } else {
            Error::Http(e)
        }
    }

    async fn check(resp: reqwest::Response) -> Result<Value> {
        if resp.status() == reqwest::StatusCode::FORBIDDEN
            || resp.status() == reqwest::StatusCode::UNAUTHORIZED
        {
            return Err(Error::Unauthorized);
        }
        let status = resp.status();
        let v: Value = resp.json().await?;
        let errors: Vec<&str> = v
            .get("errors")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|e| e.get("description").and_then(Value::as_str))
            .collect();
        if !status.is_success() || !errors.is_empty() {
            return Err(Error::Bridge(if errors.is_empty() {
                status.to_string()
            } else {
                errors.join("; ")
            }));
        }
        Ok(v)
    }

    /// Fetches every resource the bridge knows about.
    pub async fn resources(&self) -> Result<Vec<Value>> {
        let resp = self
            .http
            .get(format!("https://{}/clip/v2/resource", self.address))
            .header("hue-application-key", &self.app_key)
            .send()
            .await
            .map_err(|e| self.connection_error(e))?;
        let v = Self::check(resp).await?;
        Ok(v.get("data")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    pub async fn send(&self, req: &Request) -> Result<()> {
        let resp = self
            .http
            .put(format!(
                "https://{}/clip/v2/resource/{}/{}",
                self.address, req.rtype, req.id
            ))
            .header("hue-application-key", &self.app_key)
            .json(&req.body)
            .send()
            .await
            .map_err(|e| self.connection_error(e))?;
        Self::check(resp).await.map(drop)
    }

    /// Follows the server-sent event stream, calling `on_events` with each
    /// batch of events. Returns when the connection drops.
    pub async fn stream_events(&self, mut on_events: impl FnMut(Vec<Value>)) -> Result<()> {
        let resp = self
            .stream_http
            .get(format!("https://{}/eventstream/clip/v2", self.address))
            .header("hue-application-key", &self.app_key)
            .header("Accept", "text/event-stream")
            .send()
            .await
            .map_err(|e| self.connection_error(e))?;
        if !resp.status().is_success() {
            return Err(if resp.status().as_u16() == 403 {
                Error::Unauthorized
            } else {
                Error::Bridge(resp.status().to_string())
            });
        }
        let mut body = resp.bytes_stream();
        let mut parser = SseParser::default();
        while let Some(chunk) = body.next().await {
            for data in parser.push(&chunk?) {
                match serde_json::from_str::<Value>(&data) {
                    Ok(Value::Array(events)) => on_events(events),
                    Ok(other) => log::debug!("ignoring event payload {other}"),
                    Err(e) => log::warn!("unparseable event: {e}"),
                }
            }
        }
        Ok(())
    }
}

/// Splits a server-sent event stream into the `data` payload of each event.
#[derive(Default)]
struct SseParser {
    // Raw bytes: a chunk can end in the middle of a multi-byte character, so
    // only complete lines are decoded.
    buffer: Vec<u8>,
    data: String,
}

impl SseParser {
    fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(chunk);
        let mut out = Vec::new();
        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let raw: Vec<u8> = self.buffer.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&raw[..pos]);
            let line = line.trim_end_matches('\r');
            if line.is_empty() {
                if !self.data.is_empty() {
                    out.push(std::mem::take(&mut self.data));
                }
            } else if let Some(d) = line.strip_prefix("data:") {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(d.trim_start());
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::SseParser;

    #[test]
    fn sse_parser_handles_split_chunks() {
        let mut p = SseParser::default();
        assert!(p.push(b": hi\n\nid: 1:0\ndata: [{\"a\"").is_empty());
        let out = p.push(b":1}]\n\n");
        assert_eq!(out, vec!["[{\"a\":1}]".to_owned()]);
    }

    #[test]
    fn sse_parser_keeps_characters_split_across_chunks() {
        let mut p = SseParser::default();
        let text = "data: [\"K\u{fc}che\"]\n\n".as_bytes();
        let split = text.iter().position(|&b| b == 0xc3).unwrap() + 1;
        assert!(p.push(&text[..split]).is_empty());
        assert_eq!(p.push(&text[split..]), vec!["[\"K\u{fc}che\"]".to_owned()]);
    }

    #[test]
    fn sse_parser_handles_crlf_and_multiple_events() {
        let mut p = SseParser::default();
        let out = p.push(b"data: [1]\r\n\r\ndata: [2]\n\n");
        assert_eq!(out, vec!["[1]".to_owned(), "[2]".to_owned()]);
    }
}
