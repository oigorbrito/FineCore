//! Provider-neutral host authentication dispatch.

mod oauth_device;
mod openai_codex;
pub(crate) mod pi_compat;
mod store;

use crate::msg::Provider;
pub use openai_codex::CodexCliCredential;

/// API-key providers available to FineCore's login picker. OAuth providers
/// are included only when FineCore already has a native login implementation.
pub fn login_providers() -> Vec<Provider> {
    Provider::ALL
        .into_iter()
        .filter(|provider| {
            provider.key_env().is_some()
                || matches!(provider.auth(), crate::msg::ProviderAuth::OAuth)
        })
        .collect()
}

/// Providers for which FineCore has a locally saved API key. External auth
/// sources are not presented as FineCore-owned logout entries.
pub fn logout_providers() -> Vec<Provider> {
    Provider::ALL
        .into_iter()
        .filter(|provider| provider.stored_key().is_some())
        .collect()
}

/// Resolve a provider's API key using FineCore's explicit Pi-file adapter,
/// then its existing environment/config fallback. No command is executed.
pub fn resolve_api_key(provider: Provider) -> Result<Option<String>, String> {
    if provider.key_env().is_none() {
        return Ok(None);
    }
    let pi_key = pi_compat::configured_key(provider.label())?;
    Ok(pi_key.or_else(|| provider.resolve_key()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiKeySource {
    PiCompatibility,
    Environment,
    FineCoreConfig,
    Missing,
}

fn choose_api_key_source(
    has_pi: bool,
    has_environment: bool,
    has_finecore_config: bool,
) -> ApiKeySource {
    if has_pi {
        ApiKeySource::PiCompatibility
    } else if has_environment {
        ApiKeySource::Environment
    } else if has_finecore_config {
        ApiKeySource::FineCoreConfig
    } else {
        ApiKeySource::Missing
    }
}

/// Describe the effective source without returning or logging credential data.
pub fn api_key_source_notice(provider: Provider) -> Result<String, String> {
    let Some(environment) = provider.key_env() else {
        return Ok("this provider uses OAuth rather than an API key".into());
    };
    let has_pi = pi_compat::configured_key(provider.label())?.is_some();
    Ok(match choose_api_key_source(
        has_pi,
        provider.env_key().is_some(),
        provider.stored_key().is_some(),
    ) {
        ApiKeySource::PiCompatibility => {
            "the explicitly configured Pi credential takes precedence over the FineCore-saved key".into()
        }
        ApiKeySource::Environment => {
            format!("{environment} takes precedence over the FineCore-saved key")
        }
        ApiKeySource::FineCoreConfig => "the FineCore-saved key is the effective source".into(),
        ApiKeySource::Missing => "no effective API key source was found".into(),
    })
}

/// Store a manually entered API key using FineCore's existing per-user config.
pub fn login_api_key(provider: Provider, key: &str) -> Result<std::path::PathBuf, String> {
    if provider.key_env().is_none() {
        return Err(format!("{} does not use an API key", provider.label()));
    }
    let key = key.trim();
    if key.is_empty() {
        return Err("API key cannot be empty".into());
    }
    crate::config::save_key(provider.label(), key)
        .map_err(|error| format!("could not save API key for {}: {error}", provider.label()))
}

/// Remove only a key saved by FineCore for an API-key provider.
pub fn logout_api_key(provider: Provider) -> Result<bool, String> {
    if provider.key_env().is_none() {
        return Err(format!(
            "{} has no FineCore API-key login",
            provider.label()
        ));
    }
    crate::config::remove_key(provider.label()).map_err(|error| {
        format!(
            "could not remove saved API key for {}: {error}",
            provider.label()
        )
    })
}

pub fn status(provider: Provider) -> Result<&'static str, String> {
    match provider {
        Provider::OpenAiCodex => CodexCliCredential::discover()
            .status()
            .map_err(|error| error.message),
        _ => Err(format!("{} does not use OAuth", provider.label())),
    }
}

pub fn status_summary(provider: Provider) -> &'static str {
    match provider {
        Provider::OpenAiCodex => CodexCliCredential::discover().status_summary(),
        _ => "OAuth unavailable",
    }
}

#[cfg(test)]
mod credential_tests {
    use super::*;

    #[test]
    fn api_key_login_and_logout_are_provider_scoped() {
        login_api_key(Provider::OpenRouter, " sk-test-only ").unwrap();
        login_api_key(Provider::OpenAi, "openai-test-only").unwrap();
        assert_eq!(
            Provider::OpenRouter.stored_key().as_deref(),
            Some("sk-test-only")
        );

        assert!(logout_api_key(Provider::OpenRouter).unwrap());
        assert_eq!(Provider::OpenRouter.stored_key(), None);
        assert_eq!(
            Provider::OpenAi.stored_key().as_deref(),
            Some("openai-test-only")
        );
        assert!(!logout_api_key(Provider::OpenRouter).unwrap());
    }

    #[test]
    fn login_picker_excludes_no_auth_provider() {
        assert!(login_providers().contains(&Provider::OpenAiCodex));
        assert!(!login_providers().contains(&Provider::Local));
    }

    #[test]
    fn external_sources_take_precedence_over_the_saved_key() {
        assert_eq!(
            choose_api_key_source(true, true, true),
            ApiKeySource::PiCompatibility
        );
        assert_eq!(
            choose_api_key_source(false, true, true),
            ApiKeySource::Environment
        );
        assert_eq!(
            choose_api_key_source(false, false, true),
            ApiKeySource::FineCoreConfig
        );
        assert_eq!(
            choose_api_key_source(false, false, false),
            ApiKeySource::Missing
        );
    }
}

pub async fn login(
    provider: Provider,
    progress: impl Fn(String) + Send + Sync + 'static,
) -> Result<(), String> {
    match provider {
        Provider::OpenAiCodex => openai_codex::login(progress)
            .await
            .map(|_| ())
            .map_err(|error| error.message),
        _ => Err(format!(
            "{} does not support interactive OAuth",
            provider.label()
        )),
    }
}

pub async fn login_codex_browser(
    progress: impl Fn(String) + Send + Sync + 'static,
    show_auth_url: impl Fn(String) + Send + Sync + 'static,
    manual_code: tokio::sync::oneshot::Receiver<String>,
) -> Result<(), String> {
    openai_codex::login_browser(progress, show_auth_url, manual_code)
        .await
        .map(|_| ())
        .map_err(|error| error.message)
}
