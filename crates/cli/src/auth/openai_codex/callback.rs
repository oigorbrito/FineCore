//! Loopback listener that receives the OAuth redirect (RFC 8252 §7.3).
//! Accepts only `GET <path>` with the expected `state`. The browser page is written
//! only after the `complete` step (token exchange) finishes, so the page never reports
//! success for a login that later fails.

use reqwest::Url;
use std::future::Future;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAX_HEAD_BYTES: usize = 8 * 1024;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

/// Binds only to the loopback interface, never to `0.0.0.0`.
pub async fn bind(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(("127.0.0.1", port)).await
}

/// Waits for a valid callback, runs `complete` with its code, and answers the browser
/// with success only if `complete` succeeds. Invalid requests get an error page and the
/// wait continues.
pub async fn wait_for_callback<T, F, Fut>(
    listener: &TcpListener,
    path: &str,
    expected_state: &str,
    timeout: Duration,
    complete: F,
) -> Result<T, String>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    match tokio::time::timeout(
        timeout,
        accept_code(listener, path, expected_state, complete),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err("sign-in timed out waiting for the browser".into()),
    }
}

async fn accept_code<T, F, Fut>(
    listener: &TcpListener,
    path: &str,
    expected_state: &str,
    complete: F,
) -> Result<T, String>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|error| format!("callback listener failed: {error}"))?;
        match validate(&mut stream, path, expected_state).await {
            Check::Ignore => {}
            Check::Denied(message) => return Err(message),
            Check::Code(code) => {
                let outcome = complete(code).await;
                match outcome {
                    Ok(value) => {
                        respond(&mut stream, 200, "OK", PAGE_SIGNED_IN).await;
                        return Ok(value);
                    }
                    Err(message) => {
                        respond(&mut stream, 502, "Bad Gateway", PAGE_FAILED).await;
                        return Err(message);
                    }
                }
            }
        }
    }
}

enum Check {
    /// Not the callback; a page was already sent. Keep waiting.
    Ignore,
    /// The provider reported an error; a page was already sent.
    Denied(String),
    /// A callback with a code and the expected state.
    Code(String),
}

async fn validate(stream: &mut TcpStream, path: &str, expected_state: &str) -> Check {
    let head = match tokio::time::timeout(CONNECTION_TIMEOUT, read_head(stream)).await {
        Ok(Ok(head)) => head,
        _ => {
            respond(stream, 400, "Bad Request", PAGE_BAD_REQUEST).await;
            return Check::Ignore;
        }
    };
    let mut parts = head.lines().next().unwrap_or("").split_whitespace();
    let (method, target) = (parts.next(), parts.next());
    let url = target.and_then(|target| Url::parse(&format!("http://127.0.0.1{target}")).ok());
    let url = match (method, url) {
        (Some("GET"), Some(url)) if url.path() == path => url,
        _ => {
            respond(stream, 404, "Not Found", PAGE_NOT_FOUND).await;
            return Check::Ignore;
        }
    };
    let param = |key: &str| {
        url.query_pairs()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.into_owned())
    };
    if param("state").as_deref() != Some(expected_state) {
        respond(stream, 400, "Bad Request", PAGE_STATE_MISMATCH).await;
        return Check::Ignore;
    }
    if param("error").is_some() {
        respond(stream, 400, "Bad Request", PAGE_DENIED).await;
        return Check::Denied("authorization was denied or failed in the browser".into());
    }
    match param("code").filter(|code| !code.is_empty()) {
        Some(code) => Check::Code(code),
        None => {
            respond(stream, 400, "Bad Request", PAGE_MISSING_CODE).await;
            Check::Ignore
        }
    }
}

async fn read_head(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    while buf.len() < MAX_HEAD_BYTES {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..read]);
        if buf.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

async fn respond(stream: &mut TcpStream, status: u16, reason: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\n\
         Cache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

// Static pages only: nothing from the request is echoed back.
const PAGE_SIGNED_IN: &str =
    "<!doctype html><title>EvCore</title><p>Signed in to OpenAI. You may close this page.</p>";
const PAGE_FAILED: &str = "<!doctype html><title>EvCore</title><p>OpenAI sign-in failed. Return to EvCore for the error details.</p>";
const PAGE_DENIED: &str =
    "<!doctype html><title>EvCore</title><p>OpenAI sign-in was not completed.</p>";
const PAGE_STATE_MISMATCH: &str = "<!doctype html><title>EvCore</title><p>State mismatch.</p>";
const PAGE_MISSING_CODE: &str =
    "<!doctype html><title>EvCore</title><p>Missing authorization code.</p>";
const PAGE_NOT_FOUND: &str = "<!doctype html><title>EvCore</title><p>Callback route not found.</p>";
const PAGE_BAD_REQUEST: &str = "<!doctype html><title>EvCore</title><p>Bad request.</p>";

#[cfg(test)]
mod tests {
    use super::*;

    /// Sends one request and returns the full response text.
    async fn exchange_raw(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response).await;
        String::from_utf8_lossy(&response).into_owned()
    }

    fn get(target: &str) -> String {
        format!("GET {target} HTTP/1.1\r\nHost: localhost\r\n\r\n")
    }

    /// Runs the wait while `requests` are sent by a client task. Returns the wait result
    /// and the responses to each request, in order.
    async fn run_with<F, Fut>(
        requests: Vec<String>,
        complete: F,
    ) -> (Result<String, String>, Vec<String>)
    where
        F: FnOnce(String) -> Fut,
        Fut: Future<Output = Result<String, String>>,
    {
        let listener = bind(0).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = tokio::spawn(async move {
            let mut responses = Vec::new();
            for request in requests {
                responses.push(exchange_raw(port, &request).await);
            }
            responses
        });
        let result = wait_for_callback(
            &listener,
            "/auth/callback",
            "st",
            Duration::from_secs(5),
            complete,
        )
        .await;
        (result, client.await.unwrap())
    }

    async fn ok_complete(code: String) -> Result<String, String> {
        Ok(code)
    }

    #[tokio::test]
    async fn returns_code_and_shows_success_after_complete() {
        let (result, responses) =
            run_with(vec![get("/auth/callback?code=abc&state=st")], ok_complete).await;
        assert_eq!(result.unwrap(), "abc");
        assert!(responses[0].starts_with("HTTP/1.1 200 OK"));
    }

    #[tokio::test]
    async fn failed_complete_shows_failure_not_success() {
        let (result, responses) =
            run_with(vec![get("/auth/callback?code=abc&state=st")], |_| async {
                Err::<String, String>("token exchange failed (HTTP 400)".to_string())
            })
            .await;
        assert_eq!(result.unwrap_err(), "token exchange failed (HTTP 400)");
        assert!(responses[0].starts_with("HTTP/1.1 502"));
        assert!(!responses[0].contains("Signed in"));
    }

    #[tokio::test]
    async fn ignores_wrong_state_and_wrong_path_until_valid_callback() {
        let (result, responses) = run_with(
            vec![
                get("/auth/callback?code=evil&state=other"),
                get("/elsewhere?code=evil&state=st"),
                get("/auth/callback?code=good&state=st"),
            ],
            ok_complete,
        )
        .await;
        assert_eq!(result.unwrap(), "good");
        assert!(responses[0].starts_with("HTTP/1.1 400"));
        assert!(responses[1].starts_with("HTTP/1.1 404"));
        assert!(responses[2].starts_with("HTTP/1.1 200"));
    }

    #[tokio::test]
    async fn provider_error_fails_the_login() {
        let (result, _) = run_with(
            vec![get("/auth/callback?error=access_denied&state=st")],
            ok_complete,
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn busy_port_is_reported_as_address_in_use() {
        let held = bind(0).await.unwrap();
        let port = held.local_addr().unwrap().port();
        let second = bind(port).await;
        assert_eq!(second.unwrap_err().kind(), std::io::ErrorKind::AddrInUse);
    }

    #[tokio::test]
    async fn cancelled_wait_releases_the_port() {
        let listener = bind(0).await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let task = tokio::spawn(async move {
            wait_for_callback(
                &listener,
                "/auth/callback",
                "st",
                Duration::from_secs(300),
                ok_complete,
            )
            .await
        });
        tokio::time::sleep(Duration::from_millis(50)).await;
        task.abort();
        let _ = task.await;
        assert!(bind(port).await.is_ok());
    }

    #[tokio::test]
    async fn times_out_without_callback() {
        let listener = bind(0).await.unwrap();
        let result = wait_for_callback(
            &listener,
            "/auth/callback",
            "st",
            Duration::from_millis(50),
            ok_complete,
        )
        .await;
        assert!(result.unwrap_err().contains("timed out"));
    }
}
