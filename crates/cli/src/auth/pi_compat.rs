//! Opt-in, read-only bridge for Pi's API-key credential records.
//!
//! No automatic access to another application's secrets and no JavaScript
//! subprocesses. Pi OAuth grants require a separate provider-specific adapter.
use std::path::Path;

use serde_json::Value;

const MAX_AUTH_BYTES: u64 = 1_048_576;

/// Interpret the API-key subset of Pi's auth.json for an existing FineCore
/// provider. Neither the file contents nor returned keys are logged.
pub fn read_pi_api_key(path: &Path, provider: &str) -> Result<Option<String>, String> {
    let metadata =
        std::fs::metadata(path).map_err(|_| "cannot read FINECORE_PI_AUTH_FILE".to_string())?;
    if !metadata.is_file() || metadata.len() > MAX_AUTH_BYTES {
        return Err("FINECORE_PI_AUTH_FILE must be a file smaller than 1 MiB".into());
    }
    let bytes = std::fs::read(path).map_err(|_| "cannot read FINECORE_PI_AUTH_FILE".to_string())?;
    let root: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "FINECORE_PI_AUTH_FILE contains invalid JSON".to_string())?;
    let root = root
        .as_object()
        .ok_or("FINECORE_PI_AUTH_FILE must contain a JSON object")?;
    let Some(entry) = root.get(provider) else {
        return Ok(None);
    };
    let entry = entry
        .as_object()
        .ok_or("Pi credential entry is not a JSON object")?;
    match entry.get("type").and_then(Value::as_str) {
        Some("oauth") => Err(format!(
            "Pi OAuth credentials for {provider} cannot be used as an API key; a native OAuth adapter is required"
        )),
        Some("api_key") => {
            let Some(raw) = entry.get("key").and_then(Value::as_str) else {
                return Err(format!("Pi API-key credential for {provider} has no key"));
            };
            let raw = raw.trim();
            if raw.is_empty() {
                return Err(format!("Pi API-key credential for {provider} is blank"));
            }
            if raw.starts_with('!') {
                return Err("Pi shell-command credential references are not executed by FineCore".into());
            }
            // Pi accepts environment references in the key field. Do not
            // interpret an unresolved reference as a literal credential.
            let value = if looks_like_env_name(raw) {
                std::env::var(raw).map_err(|_| {
                    format!("Pi API-key credential for {provider} references an unset environment variable")
                })?
            } else {
                raw.to_owned()
            };
            if value.trim().is_empty() {
                return Err(format!("Pi API-key credential for {provider} resolves to an empty value"));
            }
            Ok(Some(value))
        }
        _ => Err(format!(
            "Pi credential type for {provider} is unsupported or malformed"
        )),
    }
}

/// A conservative interpretation of Pi's uppercase environment reference
/// convention. No shell evaluation or arbitrary command execution.
fn looks_like_env_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'_'))
        && bytes.all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

pub fn configured_key(provider: &str) -> Result<Option<String>, String> {
    let Some(path) = std::env::var_os("FINECORE_PI_AUTH_FILE") else {
        return Ok(None);
    };
    if path.is_empty() {
        return Err("FINECORE_PI_AUTH_FILE cannot be empty".into());
    }
    let path = Path::new(&path);
    if !path.is_absolute() {
        return Err("FINECORE_PI_AUTH_FILE must be an absolute path".into());
    }
    read_pi_api_key(path, provider)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn fixture(text: &str) -> std::path::PathBuf {
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "finecore-pi-auth-{}-{seq}.json",
            std::process::id()
        ));
        std::fs::write(&path, text).expect("write fixture");
        path
    }

    #[test]
    fn reads_a_pi_api_key_only_for_selected_provider() {
        let path = fixture(
            r#"{"openrouter":{"type":"api_key","key":"sk-test-only"},"anthropic":{"type":"api_key","key":"other"}}"#,
        );
        assert_eq!(
            read_pi_api_key(&path, "openrouter").unwrap().as_deref(),
            Some("sk-test-only")
        );
        assert_eq!(read_pi_api_key(&path, "openai").unwrap(), None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn refuses_oauth_as_api_key_without_leaking_tokens() {
        let path = fixture(
            r#"{"anthropic":{"type":"oauth","access":"sensitive","refresh":"sensitive","expires":9999999}}"#,
        );
        let error = read_pi_api_key(&path, "anthropic").unwrap_err();
        assert!(error.contains("native OAuth adapter"));
        assert!(!error.contains("sensitive"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn never_executes_shell_references() {
        let path = fixture(r#"{"openai":{"type":"api_key","key":"!echo secret"}}"#);
        let err = read_pi_api_key(&path, "openai").unwrap_err();
        assert!(err.contains("not executed"));
        assert!(!err.contains("secret"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_invalid_and_missing_files() {
        let path = fixture("[]");
        assert!(read_pi_api_key(&path, "openai")
            .unwrap_err()
            .contains("JSON object"));
        std::fs::write(&path, "{").unwrap();
        assert!(read_pi_api_key(&path, "openai")
            .unwrap_err()
            .contains("invalid JSON"));
        let _ = std::fs::remove_file(&path);
        assert!(read_pi_api_key(&path, "openai").is_err());
    }

    #[test]
    fn distinguishes_env_references_from_literal_keys() {
        assert!(looks_like_env_name("ANTHROPIC_API_KEY"));
        assert!(!looks_like_env_name("sk-abc123"));
        assert!(!looks_like_env_name("Mixed_Var"));
        assert!(!looks_like_env_name("9FOO"));
    }
}
