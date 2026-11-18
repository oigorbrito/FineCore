//! PKCE (RFC 7636, S256) verifier, challenge and anti-CSRF `state`.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::{digest, rand::SecureRandom, rand::SystemRandom};

pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

/// Fresh 32-byte verifier (43 base64url characters) and its SHA-256 challenge.
pub fn generate() -> Result<Pkce, String> {
    let mut bytes = [0u8; 32];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "system random source unavailable".to_string())?;
    let verifier = URL_SAFE_NO_PAD.encode(bytes);
    Ok(Pkce {
        challenge: challenge_for(&verifier),
        verifier,
    })
}

/// Random 16-byte `state`, hex encoded.
pub fn state() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| "system random source unavailable".to_string())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(digest::digest(&digest::SHA256, verifier.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_matches_rfc7636_appendix_b_vector() {
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn generated_values_have_expected_shape() {
        let pkce = generate().unwrap();
        assert_eq!(pkce.verifier.len(), 43);
        assert_eq!(pkce.challenge.len(), 43);
        let state = state().unwrap();
        assert_eq!(state.len(), 32);
        assert!(state.bytes().all(|b| b.is_ascii_hexdigit()));
    }
}
