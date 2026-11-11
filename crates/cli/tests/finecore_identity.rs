//! Process-level identity and updater safety checks, with no real credentials.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT: AtomicU64 = AtomicU64::new(0);

fn command(binary: &str) -> Command {
    let mut cmd = Command::new(binary);
    let root = std::env::temp_dir().join(format!(
        "finecore-identity-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    cmd.env_clear()
        .env("HOME", &root)
        .env("APPDATA", &root)
        .env("ORCA_CONFIG_DIR", root.join("config"))
        .current_dir(root);
    #[cfg(windows)]
    if let Some(system_root) = std::env::var_os("SystemRoot") {
        cmd.env("SystemRoot", system_root);
    }
    cmd
}

#[test]
fn both_executables_report_identity_and_exit_codes() {
    for (binary, name) in [
        (env!("CARGO_BIN_EXE_finecore"), "finecore"),
        (env!("CARGO_BIN_EXE_orcacode"), "orcacode"),
    ] {
        let help = command(binary).arg("--help").output().unwrap();
        assert!(help.status.success());
        assert!(String::from_utf8_lossy(&help.stdout).contains(&format!("{name} [OPTIONS]")));
        let version = command(binary).arg("--version").output().unwrap();
        assert!(version.status.success());
        assert!(String::from_utf8_lossy(&version.stdout).starts_with(&format!("{name} ")));
        let invalid = command(binary).arg("--not-a-real-option").output().unwrap();
        assert!(!invalid.status.success());
        let plugin = command(binary).arg("plugin").output().unwrap();
        assert!(!plugin.status.success());
        assert!(String::from_utf8_lossy(&plugin.stderr).contains(&format!("{name} plugin")));
        let resume = command(binary).arg("resume").output().unwrap();
        assert!(!resume.status.success());
        assert!(String::from_utf8_lossy(&resume.stderr).contains(&format!("{name} resume")));
    }
}

#[test]
fn headless_processes_send_finecore_prompt_to_local_model() {
    for (binary, bare) in [
        (env!("CARGO_BIN_EXE_finecore"), false),
        (env!("CARGO_BIN_EXE_finecore"), true),
        (env!("CARGO_BIN_EXE_orcacode"), true),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut request = Vec::new();
            let mut buf = [0; 8192];
            loop {
                let n = stream.read(&mut buf).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&buf[..n]);
                if let Some(end) = request.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&request[..end]).to_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .map(|v| v.trim().parse().unwrap())
                        })
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let body = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"synthetic-response\"}}]}\n\ndata: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
            write!(stream, "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}", body.len()).unwrap();
            String::from_utf8(request).unwrap()
        });
        let mut cmd = command(binary);
        cmd.args([
            "--provider",
            "local",
            "--base-url",
            &url,
            "--model",
            "synthetic",
            "--no-session",
            "--json",
            "-p",
            "hello",
        ]);
        if bare {
            cmd.arg("--bare");
        }
        let output = cmd.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("synthetic-response"));
        assert!(server.join().unwrap().contains("You are FineCore"));
    }
}

#[test]
fn update_and_interactive_start_do_not_contact_proxy_or_run_installer() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let proxy = format!("http://{}", listener.local_addr().unwrap());
    for binary in [
        env!("CARGO_BIN_EXE_finecore"),
        env!("CARGO_BIN_EXE_orcacode"),
    ] {
        let mut update_command = command(binary);
        // Windows searches the current directory before PATH. This harmless
        // test-harness copy prevents a regression from running the installer.
        #[cfg(windows)]
        std::fs::copy(
            std::env::current_exe().unwrap(),
            update_command
                .get_current_dir()
                .unwrap()
                .join("powershell.exe"),
        )
        .unwrap();
        let update = update_command
            .env("HTTPS_PROXY", &proxy)
            .env("HTTP_PROXY", &proxy)
            .env("PATH", "")
            .arg("update")
            .output()
            .unwrap();
        assert_eq!(update.status.code(), Some(1));
        assert!(update.stdout.is_empty());
        assert!(String::from_utf8_lossy(&update.stderr)
            .contains("update unavailable: no FineCore update channel is configured"));
        let mut child = command(binary)
            .env("HTTPS_PROXY", &proxy)
            .env("HTTP_PROXY", &proxy)
            .env("PATH", "")
            .arg("--no-session")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(3));
        #[cfg(windows)]
        assert!(
            child.try_wait().unwrap().is_none(),
            "interactive startup exited before observation"
        );
        let _ = child.kill();
        let _ = child.wait();
    }
    assert!(listener.accept().is_err(), "unexpected proxy connection");
}
