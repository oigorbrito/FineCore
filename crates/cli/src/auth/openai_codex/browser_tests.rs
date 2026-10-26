use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn matches_pi_browser_authorization_pkce_contract() {
    let flow = BrowserAuthorization::new("FineCore").unwrap();
    let url = reqwest::Url::parse(&flow.url).unwrap();
    assert_eq!(url.as_str().split('?').next(), Some(AUTH_URL));
    let args: std::collections::HashMap<String, String> =
        url.query_pairs().into_owned().collect();
    for (key, value) in [
        ("response_type", "code"),
        ("client_id", CLIENT_ID),
        ("redirect_uri", REDIRECT_URI),
        ("scope", SCOPE),
        ("code_challenge_method", "S256"),
        ("state", flow.state.as_str()),
        ("id_token_add_organizations", "true"),
        ("codex_cli_simplified_flow", "true"),
        ("originator", "FineCore"),
    ] {
        assert_eq!(args.get(key).map(String::as_str), Some(value), "{key}");
    }
    let digest = digest::digest(&digest::SHA256, flow.verifier.as_bytes());
    assert_eq!(args["code_challenge"], URL_SAFE_NO_PAD.encode(digest.as_ref()));
    assert_eq!(flow.verifier.len(), 43);
    assert_eq!(flow.state.len(), 32);
    let another = BrowserAuthorization::new("FineCore").unwrap();
    assert_ne!(flow.verifier, another.verifier);
    assert_ne!(flow.state, another.state);
    assert_eq!(port(), 1455);
}

#[test]
fn pi_manual_fallback_rejects_wrong_state_and_empty_codes() {
    for text in [
        "secret-code",
        "secret-code#state-1",
        "code=secret-code&state=state-1",
        "http://localhost:1455/auth/callback?code=secret-code&state=state-1",
    ] {
        assert_eq!(
            parse_authorization_input(text, "state-1").unwrap(),
            "secret-code"
        );
    }
    for text in [
        "",
        "code=&state=state-1",
        "secret-code#wrong",
        "code=secret-code&state=wrong",
        "http://localhost:1455/auth/callback?state=state-1",
    ] {
        assert!(parse_authorization_input(text, "state-1").is_err());
    }
}

async fn request(addr: SocketAddr, path: &str, host: &str) -> String {
    let mut socket = TcpStream::connect(addr).await.unwrap();
    socket
        .write_all(format!("GET {path} HTTP/1.1\r\nHost: {host}\r\n\r\n").as_bytes())
        .await
        .unwrap();
    let mut response = Vec::new();
    socket.read_to_end(&mut response).await.unwrap();
    String::from_utf8(response).unwrap()
}

#[tokio::test]
async fn callback_enforces_path_state_host_and_single_completion() {
    let callback = BrowserCallback::bind("good", 0).await.unwrap();
    let address = callback.addr().unwrap();
    let wait = tokio::spawn(async move { callback.wait().await });
    let host = format!("localhost:{}", address.port());
    let malformed = request(address, "/auth/callback?code=forged&state=bad", &host).await;
    assert!(malformed.contains("400 Bad Request"));
    let wrong_host = request(address, "/auth/callback?code=forged&state=good", "attacker.example").await;
    assert!(wrong_host.contains("400 Bad Request"));
    let wrong_path = request(address, "/wrong?code=forged&state=good", &host).await;
    assert!(wrong_path.contains("404 Not Found"));
    let valid = request(address, "/auth/callback?code=accepted&state=good", &host).await;
    assert!(valid.contains("200 OK"));
    assert!(!valid.contains("accepted"));
    assert_eq!(wait.await.unwrap().unwrap(), "accepted");
}

#[tokio::test]
async fn manual_input_wins_without_provider_or_model_calls() {
    let callback = BrowserCallback::bind("expected", 0).await.unwrap();
    let (send, receive) = oneshot::channel();
    send.send("manual#expected".to_string()).unwrap();
    assert_eq!(
        wait_for_code(Some(&callback), receive, "expected")
            .await
            .unwrap(),
        "manual"
    );
}

#[tokio::test]
async fn token_exchange_uses_pkce_grant_and_preserves_synthetic_account() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let flow = BrowserAuthorization::new("FineCore").unwrap();
    let expected_verifier = flow.verifier.clone();
    let token_payload = serde_json::json!({
        "https://api.openai.com/auth": {"chatgpt_account_id": "acct-synthetic"}
    });
    let token = format!(
        "head.{}.sig",
        URL_SAFE_NO_PAD.encode(token_payload.to_string().as_bytes())
    );
    let server = tokio::spawn(async move {
        let (mut client, _) = listener.accept().await.unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0_u8; 2048];
        loop {
            let n = client.read(&mut buf).await.unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..n]);
            if let Some(pos) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&bytes[..pos]).to_string();
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.split_once(':').and_then(|(key, value)| {
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                    })
                    .unwrap_or(0);
                if bytes.len() >= pos + 4 + length {
                    break;
                }
            }
        }
        let wire = String::from_utf8_lossy(&bytes).to_string();
        assert!(wire.starts_with("POST /token HTTP/1.1"));
        assert!(wire.contains("grant_type=authorization_code"));
        assert!(wire.contains(&format!("code_verifier={expected_verifier}")));
        assert!(wire.contains("code=code-fixture"));
        let body = serde_json::json!({
            "access_token": token,
            "refresh_token": "refresh-fixture",
            "expires_in": 3600
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        client.write_all(response.as_bytes()).await.unwrap();
    });
    let (credential, refresh) = flow
        .exchange_at(
            &reqwest::Client::new(),
            &format!("http://{addr}/token"),
            "code-fixture",
        )
        .await
        .unwrap();
    assert_eq!(credential.account_id, "acct-synthetic");
    assert_eq!(refresh, "refresh-fixture");
    server.await.unwrap();
}

#[tokio::test]
async fn token_exchange_failure_does_not_include_provider_response_or_secrets() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut conn, _) = listener.accept().await.unwrap();
        let mut buf = [0_u8; 2048];
        let _ = conn.read(&mut buf).await.unwrap();
        let body = "sensitive-provider-body";
        let reply = format!(
            "HTTP/1.1 401 Unauthorized\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        conn.write_all(reply.as_bytes()).await.unwrap();
    });
    let flow = BrowserAuthorization::new("FineCore").unwrap();
    let error = flow
        .exchange_at(&reqwest::Client::new(), &format!("http://{addr}/token"), "secret-code")
        .await
        .err()
        .unwrap();
    assert!(error.contains("401"));
    assert!(!error.contains("sensitive-provider-body"));
    assert!(!error.contains("secret-code"));
    server.await.unwrap();
}
