use super::*;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn mock_refresh_server(
    fail_first: bool,
) -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let count = Arc::new(AtomicUsize::new(0));
    let seen = count.clone();
    let server = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let seen = seen.clone();
            tokio::spawn(async move {
                let mut request = [0; 8192];
                let _ = socket.read(&mut request).await;
                let number = seen.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let (status, body) = if fail_first && number == 0 {
                    ("400 Bad Request", r#"{"error":"invalid_grant"}"#)
                } else {
                    (
                        "200 OK",
                        r#"{"access_token":"x.eyJleHAiOjQxMDI0NDQ4MDB9.x","refresh_token":"rotated"}"#,
                    )
                };
                let reply = format!("HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len());
                let _ = socket.write_all(reply.as_bytes()).await;
            });
        }
    });
    (format!("http://{address}/oauth/token"), count, server)
}

fn expired_fixture(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("finecore-oauth-{label}-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auth.json");
    fs::write(&path, serde_json::to_vec(&json!({"sentinel": true, "tokens": {
        "access_token": "x.eyJleHAiOjF9.x", "refresh_token": "initial", "account_id": "synthetic"
    }})).unwrap()).unwrap();
    path
}

#[tokio::test]
async fn independent_sources_reuse_rotated_token() {
    let path = expired_fixture("independent");
    let (url, count, server) = mock_refresh_server(false).await;
    let first = CodexCliCredential::new(path.clone(), url.clone());
    let second = CodexCliCredential::new(path.clone(), url);
    let (a, b) = tokio::join!(first.current(false), second.current(false));
    assert_eq!(
        a.unwrap().bearer.access_token,
        b.unwrap().bearer.access_token
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    let json: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(json["tokens"]["refresh_token"], "rotated");
    assert_eq!(json["sentinel"], true);
    server.abort();
}

#[tokio::test]
async fn forced_refresh_reuses_concurrent_rotation() {
    let path = expired_fixture("force");
    let (url, count, server) = mock_refresh_server(false).await;
    let first = CodexCliCredential::new(path.clone(), url.clone());
    let second = CodexCliCredential::new(path, url);
    let (a, b) = tokio::join!(first.current(true), second.current(true));
    assert_eq!(
        a.unwrap().bearer.access_token,
        b.unwrap().bearer.access_token
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    server.abort();
}

#[tokio::test]
async fn unauthorized_recovery_coordinates_independent_sources() {
    let path = expired_fixture("unauthorized");
    let (url, count, server) = mock_refresh_server(false).await;
    let first = CodexCliCredential::new(path.clone(), url.clone());
    let second = CodexCliCredential::new(path.clone(), url);
    let (a, b) = tokio::join!(
        first.recover_unauthorized("x.eyJleHAiOjF9.x"),
        second.recover_unauthorized("x.eyJleHAiOjF9.x")
    );
    assert_eq!(
        a.unwrap().bearer.access_token,
        b.unwrap().bearer.access_token
    );
    assert_eq!(count.load(Ordering::SeqCst), 1);
    server.abort();
}

#[tokio::test]
async fn cancelled_caller_still_persists_rotated_token() {
    let path = expired_fixture("cancelled");
    let (url, count, server) = mock_refresh_server(false).await;
    let first = CodexCliCredential::new(path.clone(), url.clone());
    let task = tokio::spawn(async move { first.current(false).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while count.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    task.abort();
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["tokens"]["refresh_token"], "rotated");
    let second = CodexCliCredential::new(path, url);
    assert!(second.current(false).await.is_ok());
    assert_eq!(count.load(Ordering::SeqCst), 1);
    server.abort();
}

#[tokio::test]
async fn failed_refresh_releases_lock_for_retry() {
    let path = expired_fixture("failure");
    let (url, count, server) = mock_refresh_server(true).await;
    let first = CodexCliCredential::new(path.clone(), url.clone());
    let second = CodexCliCredential::new(path.clone(), url);
    assert!(first.current(false).await.is_err());
    let unchanged: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(unchanged["tokens"]["refresh_token"], "initial");
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(5), second.current(false))
            .await
            .unwrap()
            .is_ok()
    );
    assert_eq!(count.load(Ordering::SeqCst), 2);
    let json: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(json["tokens"]["refresh_token"], "rotated");
    server.abort();
}

#[test]
fn decodes_expiry_and_distinguishes_malformed_auth() {
    assert_eq!(jwt_exp("x.eyJleHAiOjEyMzQ1fQ.y"), Some(12345));
    let path = std::env::temp_dir().join(format!("orcacode-auth-bad-{}", std::process::id()));
    fs::write(&path, b"not-json").unwrap();
    assert_eq!(
        read_auth(&path).unwrap_err().kind,
        CredentialErrorKind::Malformed
    );
    fs::remove_file(path).unwrap();
}

#[test]
fn refresh_contract_matches_official_codex_json_shape() {
    assert_eq!(
        refresh_form("refresh"),
        [
            ("client_id", "app_EMoamEEZ73f0CkXaXp7hrann"),
            ("grant_type", "refresh_token"),
            ("refresh_token", "refresh"),
        ]
    );
    assert!(needs_refresh(Some(1)));
    assert!(!needs_refresh(None));
}

#[test]
fn missing_auth_is_distinct_and_actionable() {
    let path = std::env::temp_dir().join(format!("orcacode-auth-missing-{}", std::process::id()));
    let error = read_auth(&path).unwrap_err();
    assert_eq!(error.kind, CredentialErrorKind::Missing);
}

#[tokio::test]
async fn unauthorized_recovery_reuses_a_concurrently_rotated_token() {
    let source = CodexCliCredential::new(PathBuf::from("/not/read"), "http://not-called".into());
    *source.state.lock().await = Some(CodexCredential {
        bearer: BearerCredential {
            access_token: "new".into(),
            expires_at: None,
        },
        account_id: "acct".into(),
    });
    assert_eq!(
        source
            .recover_unauthorized("old")
            .await
            .unwrap()
            .bearer
            .access_token,
        "new"
    );
}

#[test]
fn atomic_persistence_preserves_unknown_fields() {
    let dir = std::env::temp_dir().join(format!("orcacode-auth-dir-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auth.json");
    let root = json!({"future": true, "tokens": {"access_token":"old", "refresh_token":"refresh", "account_id":"acct"}});
    fs::write(&path, serde_json::to_vec(&root).unwrap()).unwrap();
    let renewed = CodexCredential {
        bearer: BearerCredential {
            access_token: "new".into(),
            expires_at: None,
        },
        account_id: "acct".into(),
    };
    persist_tokens(&path, root, &renewed, "refresh").unwrap();
    let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["future"], true);
    assert_eq!(saved["tokens"]["access_token"], "new");
    fs::remove_dir_all(dir).unwrap();
}
