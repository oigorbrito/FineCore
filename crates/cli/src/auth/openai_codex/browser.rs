//! Pi-equivalent OpenAI Codex browser authorization primitives in Rust.
//! The pre-existing device-code login and existing credential paths are unchanged.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ring::{digest, rand::{SecureRandom, SystemRandom}};
use std::{net::SocketAddr, time::Duration};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::oneshot};

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const CALLBACK_PATH: &str = "/auth/callback";
const CALLBACK_PORT: u16 = 1455;
const SCOPE: &str = "openid profile email offline_access";
const HEADER_LIMIT: usize = 8192;

pub(super) struct BrowserAuthorization {
    pub verifier: String,
    pub state: String,
    pub url: String,
}

impl BrowserAuthorization {
    pub fn new(originator: &str) -> Result<Self, String> {
        let rng = SystemRandom::new();
        let mut secret = [0_u8; 32];
        let mut nonce = [0_u8; 16];
        rng.fill(&mut secret).map_err(|_| "secure PKCE entropy unavailable")?;
        rng.fill(&mut nonce).map_err(|_| "secure OAuth state entropy unavailable")?;
        let verifier = URL_SAFE_NO_PAD.encode(secret);
        let challenge = URL_SAFE_NO_PAD.encode(
            digest::digest(&digest::SHA256, verifier.as_bytes()).as_ref(),
        );
        let state = nonce.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        let mut url = reqwest::Url::parse(AUTH_URL).map_err(|_| "invalid OAuth authorization URL")?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", CLIENT_ID)
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("scope", SCOPE)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state)
            .append_pair("id_token_add_organizations", "true")
            .append_pair("codex_cli_simplified_flow", "true")
            .append_pair("originator", originator);
        Ok(Self { verifier, state, url: url.into() })
    }

    /// Same grant contract as the pinned Pi donor. URL is injectable only to
    /// support an offline mock server; server errors never include secrets.
    pub async fn exchange_at(
        &self, client: &reqwest::Client, token_url: &str, code: &str,
    ) -> Result<(super::CodexCredential, String), String> {
        let response = client.post(token_url).timeout(Duration::from_secs(15))
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", CLIENT_ID),
                ("code", code),
                ("code_verifier", self.verifier.as_str()),
                ("redirect_uri", REDIRECT_URI),
            ])
            .send().await.map_err(|_| "Codex token endpoint unavailable")?;
        if !response.status().is_success() {
            return Err(format!("Codex browser token exchange rejected (HTTP {})", response.status()));
        }
        let token: serde_json::Value = response.json().await
            .map_err(|_| "Codex browser token exchange returned invalid JSON")?;
        let access = token["access_token"].as_str().filter(|s| !s.is_empty())
            .ok_or("Codex token exchange missing access_token")?;
        let refresh = token["refresh_token"].as_str().filter(|s| !s.is_empty())
            .ok_or("Codex token exchange missing refresh_token")?;
        let account_id = super::jwt_account_id(access)
            .ok_or("Codex access token did not identify a ChatGPT account")?;
        Ok((
            super::CodexCredential {
                bearer: orca_harness_provider_auth::BearerCredential {
                    access_token: access.into(), expires_at: super::jwt_exp(access),
                },
                account_id,
            },
            refresh.into(),
        ))
    }
}

/// Accept exactly the input formats accepted by Pi: code, code#state,
/// code=...&state=..., and a full redirect URL. Present state must match.
pub(super) fn parse_authorization_input(input: &str, expected: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() { return Err("missing OAuth authorization code".into()); }
    let (code, state) = if let Ok(url) = reqwest::Url::parse(input) {
        let pair = |k: &str| url.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.into_owned());
        (pair("code"), pair("state"))
    } else if let Some((code, state)) = input.split_once('#') {
        (Some(code.into()), Some(state.into()))
    } else if input.contains("code=") {
        let url = reqwest::Url::parse(&format!("http://localhost/?{input}"))
            .map_err(|_| "invalid OAuth response parameters")?;
        let pair = |k: &str| url.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.into_owned());
        (pair("code"), pair("state"))
    } else {
        (Some(input.into()), None)
    };
    if state.as_deref().is_some_and(|s| s != expected) {
        return Err("OAuth state mismatch".into());
    }
    code.filter(|s| !s.is_empty()).ok_or_else(|| "missing OAuth authorization code".into())
}

pub(super) struct BrowserCallback {
    listener: TcpListener,
    expected_state: String,
}

impl BrowserCallback {
    pub async fn bind(state: &str, port: u16) -> std::io::Result<Self> {
        Ok(Self {
            listener: TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await?,
            expected_state: state.into(),
        })
    }

    #[cfg(test)]
    pub fn addr(&self) -> std::io::Result<SocketAddr> { self.listener.local_addr() }

    pub async fn wait(&self) -> Result<String, String> {
        loop {
            let (mut stream, peer) = self.listener.accept().await
                .map_err(|_| "OAuth callback accept failed")?;
            if !peer.ip().is_loopback() { continue; }
            match read_callback(&mut stream, &self.expected_state).await {
                Ok(code) => {
                    respond(&mut stream, 200, "Login received; you may close this tab").await;
                    return Ok(code);
                }
                Err((status, message)) => respond(&mut stream, status, message).await,
            }
        }
    }
}

async fn read_callback(
    stream: &mut TcpStream, expected_state: &str,
) -> Result<String, (u16, &'static str)> {
    let mut buf = Vec::with_capacity(1024);
    while buf.len() < HEADER_LIMIT && !buf.windows(4).any(|x| x == b"\r\n\r\n") {
        let mut next = [0u8; 1024];
        let n = tokio::time::timeout(Duration::from_secs(5), stream.read(&mut next)).await
            .map_err(|_| (408, "OAuth callback timed out"))?
            .map_err(|_| (400, "OAuth callback read failed"))?;
        if n == 0 { return Err((400, "Incomplete OAuth callback")); }
        buf.extend_from_slice(&next[..n]);
    }
    if !buf.windows(4).any(|x| x == b"\r\n\r\n") {
        return Err((413, "OAuth callback request too large"));
    }
    let request = std::str::from_utf8(&buf).map_err(|_| (400, "Invalid OAuth callback encoding"))?;
    let mut lines = request.split("\r\n");
    let mut first = lines.next().unwrap_or_default().split_whitespace();
    let (Some("GET"), Some(path), Some(version)) = (first.next(), first.next(), first.next()) else {
        return Err((405, "Invalid callback method"));
    };
    if !version.starts_with("HTTP/1.") || first.next().is_some() {
        return Err((400, "Invalid HTTP request line"));
    }
    let host = lines.find_map(|l| l.split_once(':')
        .and_then(|(k, v)| k.eq_ignore_ascii_case("host").then_some(v.trim())))
        .ok_or((400, "Missing callback host"))?;
    let port = stream.local_addr().map_err(|_| (400, "Invalid callback listener"))?.port();
    if host != format!("localhost:{port}") && host != format!("127.0.0.1:{port}") {
        return Err((400, "Unexpected callback host"));
    }
    let url = reqwest::Url::parse(&format!("http://localhost{path}"))
        .map_err(|_| (400, "Invalid callback URL"))?;
    if url.path() != CALLBACK_PATH { return Err((404, "Unexpected callback route")); }
    let value = |k: &str| url.query_pairs().find(|(key, _)| key == k).map(|(_, v)| v.into_owned());
    if value("state").as_deref() != Some(expected_state) {
        return Err((400, "OAuth state mismatch"));
    }
    value("code").filter(|v| !v.is_empty()).ok_or((400, "Missing OAuth authorization code"))
}

async fn respond(stream: &mut TcpStream, status: u16, message: &str) {
    let reason = match status {
        200 => "OK", 400 => "Bad Request", 404 => "Not Found",
        405 => "Method Not Allowed", 408 => "Request Timeout",
        413 => "Content Too Large", _ => "Internal Server Error",
    };
    let wire = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{message}",
        message.len(),
    );
    let _ = stream.write_all(wire.as_bytes()).await;
    let _ = stream.shutdown().await;
}

/// Manual and callback completion compete. The TUI owns the sender so a
/// cancelled attempt cannot be completed by a stale pasted credential.
pub(super) async fn wait_for_code(
    callback: Option<&BrowserCallback>, manual: oneshot::Receiver<String>, state: &str,
) -> Result<String, String> {
    let manual_input = async {
        let raw = manual.await.map_err(|_| "OAuth login cancelled")?;
        parse_authorization_input(&raw, state)
    };
    match callback {
        Some(callback) => tokio::select! {
            result = callback.wait() => result,
            result = manual_input => result,
        },
        None => manual_input.await,
    }
}

pub(super) const fn port() -> u16 { CALLBACK_PORT }

#[cfg(test)]
#[path = "browser_tests.rs"]
mod tests;
