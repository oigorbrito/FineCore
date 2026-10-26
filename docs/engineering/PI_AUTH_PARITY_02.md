# Pi auth parity 02: interactive credential management

This follow-up builds on the opt-in, read-only Pi API-key adapter in PR #120. It adds FineCore `/login` and `/logout` flows without changing the Rust provider engine or claiming full Pi auth parity.

## Implemented

- `/login` opens a provider picker. API-key providers accept masked input and save only to FineCore's existing `config.json`; the active provider is not changed. `/login <provider>` supports the same flow directly.
- `/login openai-codex` invokes FineCore's existing native Codex OAuth flow. It does not select Codex as the active provider.
- `/logout` lists only API-key credentials saved in FineCore. Selecting one removes only that provider's `api_keys` entry; it leaves other saved keys, environment variables, imported Pi credentials, and Codex-owned credentials untouched. `/logout <provider>` is also supported.
- If the logged-out provider is active, its runtime endpoint is rebuilt using any remaining configured auth source. No provider/model request is made by login, logout, or this refresh.
- The existing `/provider` setup path remains available. It shares the masked key entry and persistence behavior.

API-key logout is deliberately scoped to credentials FineCore owns. It is not a provider-wide revocation and does not revoke a token at the provider. Codex logout is not implemented here because Codex owns its credential lifecycle.

## Credential source and security boundaries

The runtime API-key resolution order is the explicit command-line key, then the opt-in Pi compatibility file, then the existing FineCore environment-variable/config resolution. This intentionally differs from Pi's original credential precedence; FineCore's existing environment-over-config rule is preserved. The Pi file is never read unless explicitly configured, and no Pi file is modified.

New API keys use FineCore's existing config store, not a new token database. On Unix, the existing writer creates/restricts the config file to owner-only mode (`0600`). On Windows, it uses ordinary filesystem writes and does not establish an explicit ACL in this code. Therefore this change does **not** claim Windows ACL parity, encrypted-at-rest storage, or a dedicated OS keychain. Windows credential-storage hardening remains a separate requirement before claiming equivalent secure persistence on that platform.

Keys are masked in the TUI while entered; rendered provider/logout rows expose status only, not credential contents. Errors and confirmations identify the provider and config path but do not include key material. Provider tests use local test values and perform no network authentication.

## Scope and remaining gaps

This is a first interactive manager, not the complete Pi login experience. FineCore currently supports API-key entry for providers in its existing catalog and its existing native OpenAI Codex OAuth. It does not add Pi's Claude, GitHub Copilot, Google, or other provider-specific OAuth flows, device flows, refresh-token management, provider catalog, or provider/model parity. Logout does not revoke external credentials. Pi-auth import remains opt-in and read-only.

No claims are made about full Pi parity. Keep the implementation Draft until provider-specific flows, secure persistence on each supported OS, and user-facing requirements have independent review.

## Validation

- `cargo fmt --all -- --check`
- `cargo check -p finecore --locked`
- Focused auth and TUI tests; the full FineCore test binary was also attempted on Windows. Existing platform-sensitive tests failed there (including path separators, file permissions, plugin/image handling, and unrelated shell-policy behavior); the newly added auth and login/logout TUI tests passed. Linux CI is required for the authoritative full suite.

The login/logout tests exercise local storage and UI command routing only. They do not invoke a model, provider, or OAuth endpoint.
