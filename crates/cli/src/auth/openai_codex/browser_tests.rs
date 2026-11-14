use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[test]
fn matches_pi_browser_authorization_pkce_contract() {
    let flow = BrowserAuthorization::new("FineCore").unwrap();
    let url = reqwest::Url::parse(&flow.url).unwrap();
    assert_eq!(url.as_str().split('?').next(), Some(AUTH_URL));
    let args: std::collections::HashMap<String, String> = url.query_pairs().into_owned().collect();
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
    assert_eq!(
        args["code_challenge"],
        URL_SAFE_NO_PAD.encode(digest.as_ref())
    );
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
        "http://localhost:1455/auth/callback?code=secret-code",
        "code=secret-code",
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
    let wrong_host = request(
        address,
        "/auth/callback?code=forged&state=good",
        "attacker.example",
    )
    .await;
    assert!(wrong_host.contains("400 Bad Request"));
    let wrong_path = request(address, "/wrong?code=forged&state=good", &host).await;
    assert!(wrong_path.contains("404 Not Found"));
    let valid = request(address, "/auth/callback?code=accepted&state=good", &host).await;
    assert!(valid.contains("200 OK"));
    assert!(!valid.contains("accepted"));
    assert_eq!(wait.await.unwrap().unwrap(), "accepted");
}

#[tokio::test]
async fn declined_authorization_returns_an_error_without_disclosing_provider_details() {
    let callback = BrowserCallback::bind("expected", 0).await.unwrap();
    let address = callback.addr().unwrap();
    let wait = tokio::spawn(async move { callback.wait().await });
    let response = request(
        address,
        "/auth/callback?error=access_denied&error_description=secret&state=expected",
        &format!("localhost:{}", address.port()),
    )
    .await;
    assert!(response.contains("403 Forbidden"));
    assert!(!response.contains("secret"));
    assert_eq!(
        wait.await.unwrap().unwrap_err(),
        "OpenAI authorization was declined"
    );
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
async fn dropped_manual_sender_cancels_and_releases_callback() {
    let callback = BrowserCallback::bind("expected", 0).await.unwrap();
    let address = callback.addr().unwrap();
    let (sender, receiver) = oneshot::channel();
    drop(sender);
    assert_eq!(
        wait_for_code(Some(&callback), receiver, "expected")
            .await
            .unwrap_err(),
        "OAuth login cancelled"
    );
    drop(callback);
    assert!(TcpListener::bind(address).await.is_ok());
}

#[tokio::test]
async fn occupied_port_allows_manual_fallback() {
    let blocker = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = blocker.local_addr().unwrap().port();
    assert!(BrowserCallback::bind("expected", port).await.is_err());
    let (sender, receiver) = oneshot::channel();
    sender.send("synthetic-code#expected".into()).unwrap();
    assert_eq!(
        wait_for_code(None, receiver, "expected").await.unwrap(),
        "synthetic-code"
    );
}

#[tokio::test]
async fn missing_callback_and_manual_input_times_out() {
    let (_sender, receiver) = oneshot::channel();
    assert_eq!(
        wait_for_code_with_timeout(None, receiver, "expected", Duration::from_millis(10))
            .await
            .unwrap_err(),
        "OAuth login timed out"
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
    let dir = std::env::temp_dir().join(format!(
        "finecore-browser-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auth.json");
    std::fs::write(&path, r#"{"tokens":{},"other":"preserved"}"#).unwrap();
    super::super::complete_browser_login(
        &flow,
        "code-fixture",
        &format!("http://{addr}/token"),
        &path,
    )
    .await
    .unwrap();
    let (root, credential, refresh) = super::super::read_auth(&path).unwrap();
    assert_eq!(root["other"], "preserved");
    assert_eq!(credential.account_id, "acct-synthetic");
    assert_eq!(refresh.as_deref(), Some("refresh-fixture"));
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "auth::openai_codex::browser::tests::browser_restart_child",
            "--test-threads=1",
        ])
        .env("FINECORE_TEST_BROWSER_AUTH_FILE", &path)
        .output()
        .unwrap();
    assert!(child.status.success(), "credential restart child failed");
    let output = String::from_utf8_lossy(&child.stdout);
    assert!(
        output.contains("1 passed"),
        "credential restart child did not run"
    );
    std::fs::remove_dir_all(dir).unwrap();
    server.await.unwrap();
}

#[test]
fn browser_restart_child() {
    if let Some(path) = std::env::var_os("FINECORE_TEST_BROWSER_AUTH_FILE") {
        let (_, credential, refresh) =
            super::super::read_auth(std::path::Path::new(&path)).unwrap();
        assert_eq!(credential.account_id, "acct-synthetic");
        assert_eq!(refresh.as_deref(), Some("refresh-fixture"));
    }
}

async fn synthetic_token_server() -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut client, _) = listener.accept().await.unwrap();
        let mut request = [0_u8; 4096];
        let _ = client.read(&mut request).await.unwrap();
        let payload = serde_json::json!({"https://api.openai.com/auth": {"chatgpt_account_id": "acct-synthetic"}});
        let token = format!(
            "head.{}.sig",
            URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes())
        );
        let body =
            serde_json::json!({"access_token": token, "refresh_token": "fake-refresh"}).to_string();
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
        client.write_all(response.as_bytes()).await.unwrap();
    });
    (format!("http://{address}/token"), server)
}

#[tokio::test]
async fn complete_browser_login_uses_manual_fallback_when_port_is_occupied() {
    let blocker = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = blocker.local_addr().unwrap().port();
    let (url, server) = synthetic_token_server().await;
    let dir = std::env::temp_dir().join(format!("finecore-browser-flow-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auth.json");
    let (sender, receiver) = oneshot::channel();
    sender.send("synthetic-code".to_string()).unwrap();
    let shown = std::cell::RefCell::new(String::new());
    let opened = std::cell::RefCell::new(String::new());
    super::super::run_browser_login(
        |value| *shown.borrow_mut() = value,
        receiver,
        port,
        &url,
        &path,
        |value| *opened.borrow_mut() = value.to_string(),
    )
    .await
    .unwrap();
    assert_eq!(*shown.borrow(), *opened.borrow());
    assert!(shown.borrow().starts_with(AUTH_URL));
    assert_eq!(
        super::super::read_auth(&path).unwrap().1.account_id,
        "acct-synthetic"
    );
    server.await.unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn callback_exchange_and_persist_complete_with_mock_server() {
    let flow = BrowserAuthorization::new("FineCore").unwrap();
    let callback = BrowserCallback::bind(&flow.state, 0).await.unwrap();
    let address = callback.addr().unwrap();
    let state = flow.state.clone();
    let (sender, receiver) = oneshot::channel::<String>();
    let send_callback = tokio::spawn(async move {
        request(
            address,
            &format!("/auth/callback?code=synthetic&state={state}"),
            &format!("localhost:{}", address.port()),
        )
        .await
    });
    let code = wait_for_code(Some(&callback), receiver, &flow.state)
        .await
        .unwrap();
    let response = send_callback.await.unwrap();
    assert!(response.contains("Authorization received"));
    assert!(!response.contains("Signed in"));
    drop(sender);
    drop(callback);
    let (url, server) = synthetic_token_server().await;
    let dir = std::env::temp_dir().join(format!("finecore-browser-full-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auth.json");
    super::super::complete_browser_login(&flow, &code, &url, &path)
        .await
        .unwrap();
    let (_, credential, _) = super::super::read_auth(&path).unwrap();
    assert_eq!(credential.account_id, "acct-synthetic");
    server.await.unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn malformed_existing_credentials_are_not_overwritten() {
    let dir = std::env::temp_dir().join(format!("finecore-browser-invalid-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auth.json");
    std::fs::write(&path, b"{invalid").unwrap();
    let (url, server) = synthetic_token_server().await;
    let flow = BrowserAuthorization::new("FineCore").unwrap();
    let result = super::super::complete_browser_login(&flow, "fake-code", &url, &path).await;
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"{invalid");
    server.await.unwrap();
    std::fs::remove_dir_all(dir).unwrap();
}

#[tokio::test]
async fn persistence_failure_is_reported_after_exchange() {
    // The existing private writer rejects non-absolute paths after the exchange.
    let path = std::path::PathBuf::from(format!(
        "finecore-browser-invalid-relative-{}",
        std::process::id()
    ));
    let (url, server) = synthetic_token_server().await;
    let flow = BrowserAuthorization::new("FineCore").unwrap();
    let error = super::super::complete_browser_login(&flow, "fake-code", &url, &path)
        .await
        .unwrap_err();
    assert!(error.message.contains("could not persist"));
    assert!(!path.exists());
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
        .exchange_at(
            &reqwest::Client::new(),
            &format!("http://{addr}/token"),
            "secret-code",
        )
        .await
        .err()
        .unwrap();
    assert!(error.contains("401"));
    assert!(!error.contains("sensitive-provider-body"));
    assert!(!error.contains("secret-code"));
    server.await.unwrap();
}
