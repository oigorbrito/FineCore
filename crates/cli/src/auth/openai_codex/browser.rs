//! Browser sign-in for OpenAI Codex: Authorization Code with PKCE (S256) and a
//! loopback redirect. The method follows the Pi coding agent (MIT, Mario Zechner,
//! `packages/ai/src/auth/oauth/openai-codex.ts`, commit 05fd3fb); it is reimplemented
//! here and no source text is copied.

use super::{callback, exchange_authorization_code, pkce, CLIENT_ID};
use orca_harness_model_providers::openai_codex::CodexCredential;
use orca_harness_provider_auth::{CredentialError, CredentialErrorKind};
use reqwest::Url;
use std::time::Duration;
use tokio::net::TcpListener;

const AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const CALLBACK_PORT: u16 = 1455;
const CALLBACK_PATH: &str = "/auth/callback";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const SCOPE: &str = "openid profile email offline_access";
const ORIGINATOR: &str = "evcore";

/// A login whose loopback listener is already bound and whose URL is ready.
pub(super) struct Pending {
    listener: TcpListener,
    url: String,
    verifier: String,
    state: String,
}

/// Binds the callback port and prepares the authorization URL.
/// An error here means the browser method is unavailable (for example, port 1455 is busy).
pub(super) async fn start() -> Result<Pending, String> {
    let pkce = pkce::generate()?;
    let state = pkce::state()?;
    let listener = callback::bind(CALLBACK_PORT)
        .await
        .map_err(|error| format!("port {CALLBACK_PORT} unavailable: {error}"))?;
    Ok(Pending {
        listener,
        url: authorize_url(&pkce.challenge, &state),
        verifier: pkce.verifier,
        state,
    })
}

impl Pending {
    pub(super) async fn complete(
        self,
        client: &reqwest::Client,
        progress: &impl Fn(String),
    ) -> Result<CodexCredential, CredentialError> {
        let client = client.clone();
        progress(format!(
            "sign in at {} · waiting for the browser on port {CALLBACK_PORT}",
            self.url
        ));
        open_browser(&self.url);
        let verifier = self.verifier.clone();
        // The browser sees success only after the token exchange succeeds.
        callback::wait_for_callback(
            &self.listener,
            CALLBACK_PATH,
            &self.state,
            CALLBACK_TIMEOUT,
            |code| async move {
                progress("authorization approved · exchanging credentials…".into());
                exchange_authorization_code(&client, &code, &verifier, REDIRECT_URI)
                    .await
                    .map_err(|error| error.message)
            },
        )
        .await
        .map_err(|message| CredentialError::new(CredentialErrorKind::Refresh, message))
    }
}

fn authorize_url(challenge: &str, state: &str) -> String {
    let mut url = Url::parse(AUTHORIZE_URL).expect("static authorize URL");
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", CLIENT_ID)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("scope", SCOPE)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        .append_pair("id_token_add_organizations", "true")
        .append_pair("codex_cli_simplified_flow", "true")
        .append_pair("originator", ORIGINATOR);
    url.into()
}

/// Best effort: the URL is always shown to the user, so a failed launch is not an error.
fn open_browser(url: &str) {
    #[cfg(windows)]
    let spawned = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let spawned = std::process::Command::new("xdg-open").arg(url).spawn();
    let _ = spawned;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_carries_pkce_state_and_redirect() {
        let url = Url::parse(&authorize_url("chal", "st")).unwrap();
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(pairs["code_challenge"], "chal");
        assert_eq!(pairs["code_challenge_method"], "S256");
        assert_eq!(pairs["state"], "st");
        assert_eq!(pairs["redirect_uri"], REDIRECT_URI);
        assert_eq!(pairs["client_id"], CLIENT_ID);
        assert_eq!(pairs["response_type"], "code");
    }
}
