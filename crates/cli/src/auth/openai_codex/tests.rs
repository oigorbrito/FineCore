use super::*;

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

#[test]
fn browser_login_replaces_only_codex_tokens_in_the_existing_owned_file() {
    let dir = std::env::temp_dir().join(format!("finecore-codex-login-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auth.json");
    fs::write(
        &path,
        br#"{"auth_mode":"chatgpt","future_field":{"keep":true},"tokens":{"extension":"preserve","access_token":"old","refresh_token":"old-refresh","account_id":"old-account"}}"#,
    )
    .unwrap();
    let credential = CodexCredential {
        bearer: BearerCredential {
            access_token: "new-access".into(),
            expires_at: Some(1_700_000_000),
        },
        account_id: "new-account".into(),
    };

    persist_login_tokens(&path, &credential, "new-refresh").unwrap();

    let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["future_field"]["keep"], true);
    assert_eq!(saved["tokens"]["extension"], "preserve");
    assert_eq!(saved["tokens"]["access_token"], "new-access");
    assert_eq!(saved["tokens"]["refresh_token"], "new-refresh");
    assert_eq!(saved["tokens"]["account_id"], "new-account");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn browser_login_does_not_replace_malformed_existing_auth_file() {
    let path =
        std::env::temp_dir().join(format!("finecore-codex-malformed-{}", std::process::id()));
    fs::write(&path, b"not-json").unwrap();
    let credential = CodexCredential {
        bearer: BearerCredential {
            access_token: "new-access".into(),
            expires_at: None,
        },
        account_id: "new-account".into(),
    };
    let error = persist_login_tokens(&path, &credential, "new-refresh").unwrap_err();
    assert_eq!(error.kind, CredentialErrorKind::Malformed);
    assert_eq!(fs::read(&path).unwrap(), b"not-json");
    fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn refresh_rotates_persisted_tokens_without_dropping_other_fields() {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (socket, _) = listener.accept().await.unwrap();
        let mut reader = BufReader::new(socket);
        let mut content_length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await.unwrap();
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    content_length = value.trim().parse::<usize>().unwrap();
                }
            }
        }
        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).await.unwrap();
        assert!(String::from_utf8(body)
            .unwrap()
            .contains("refresh_token=old-refresh"));
        let mut socket = reader.into_inner();
        let response =
            r#"{"access_token":"new-access","refresh_token":"rotated-refresh","expires_in":3600}"#;
        let wire = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
            response.len()
        );
        socket.write_all(wire.as_bytes()).await.unwrap();
    });

    let dir = std::env::temp_dir().join(format!("finecore-codex-refresh-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("auth.json");
    fs::write(
        &path,
        br#"{"auth_mode":"chatgpt","future_field":"preserve","tokens":{"access_token":"x.eyJleHAiOjF9.y","refresh_token":"old-refresh","account_id":"acct","extension":"keep"}}"#,
    )
    .unwrap();
    let source = CodexCliCredential::new(path.clone(), format!("http://{address}/token"));

    let refreshed = source.credential().await.unwrap();
    server.await.unwrap();

    assert_eq!(refreshed.access_token, "new-access");
    let saved: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["future_field"], "preserve");
    assert_eq!(saved["tokens"]["extension"], "keep");
    assert_eq!(saved["tokens"]["refresh_token"], "rotated-refresh");
    fs::remove_dir_all(dir).unwrap();
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
