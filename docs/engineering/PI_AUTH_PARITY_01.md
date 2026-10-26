# FineCore — PI-AUTH-PARITY-01

**2026-10-26.** Draft, isolated from `main`. FineCore Rust integration base: `2f980015a733ec449ca1ea8be51e97da08220fb1`. Source-of-truth Pi: [earendil-works/pi](https://github.com/earendil-works/pi), snapshot `8b0caefc2260a28d2cec0a7ce3daef1d3de24776`, `@earendil-works/pi-ai` **1.4.0** at this audited snapshot. The older `badlogic/pi-mono` address is a redirect, and the older `@mariozechner/*` packages have been deprecated; do not silently import historic versions.

## Goal and current limitations

Requested target is the complete Pi-style provider login UX — `/login`, `/logout`, API keys, subscription/device/browser OAuth, automatic refresh, protected credential store, and compatible provider+model selection — while preserving the native Orca/FineCore agent runtime and CLI behavior. **This PR does not claim complete parity or actual OAuth support beyond FineCore's existing OpenAI Codex login.**

The source FineCore integration currently provides:

- Seven provider presets: OpenRouter, Vercel, CheaperInference, OpenAI API, OpenAI Codex, Anthropic, Local.
- `/provider` with masked API-key input, environment and `config.json` keys; `/models`; an existing native Codex device login/token refresh.
- `crates/provider-auth` is intentionally a narrow provider-neutral credential-source boundary. Host-specific storage and OAuth belong in `crates/cli/src/auth`. Do **not** import the Pi TypeScript AI execution engine into the Rust kernel.
- It does **not** have Pi-equivalent `/login` / `/logout`, per-provider OAuth methods for Anthropic/GitHub Copilot/Gemini CLI/Antigravity, a full Pi model catalog, or the same credential store semantics.

Pi's own `packages/ai/src/auth/{types,resolve}.ts` defines provider-scoped API key/OAuth credentials, app-owned `CredentialStore`, stored-key precedence, lock-protected refresh with token rotation and explicit cancellation behavior. Pi `auth.json` is sensitive; it must not be uploaded to CI or automatically read by an unrelated app.

## Implemented in this first bounded slice

**Explicit, read-only Pi API-key compatibility** for the seven existing FineCore provider names that take keys, with no additional runtime/provider packages:

1. Set `FINECORE_PI_AUTH_FILE` to an **absolute path** of an existing Pi-format `auth.json`. No default Pi path is probed; the user's secrets are never imported or copied automatically.
2. FineCore run startup resolves `--api-key` first, then an explicitly opted-in Pi API-key entry for the selected provider, then the existing environment/config fallback when no Pi entry exists. No key is displayed. Existing behavior is unchanged when the environment variable is absent.
3. Missing/malformed/unsupported entry fails startup clearly; OAuth entries are **not** misrepresented as API keys. Shell-command entries beginning with `!` are rejected rather than executed; uppercase environment references are supported without invoking shells. Credential values do not appear in errors.
4. Files are only read, with a maximum 1 MiB bound. No storage rewrite, token refresh, provider registry change, model transport change or product code beyond this host adapter and its startup call.

For PowerShell, explicitly opt in **only for the current shell**:

```powershell
$env:FINECORE_PI_AUTH_FILE = Join-Path $HOME '.pi\agent\auth.json'
finecore --model gpt-4o-mini -p 'Hello'
```

That example applies to a provider already selectable in FineCore (OpenAI API). It does not authorize creating tokens, logging into subscriptions, or sending them to a third party. Choosing another provider in the existing interactive picker currently uses FineCore's pre-existing auth mechanism rather than this startup-only import; dynamic parity remains pending.

## Parity work still required (separate proof gates)

| Feature | FineCore currently | Pi behavior | Acceptance proof |
|---|---|---|---|
| Provider login menu | /provider, seven presets | /login [provider], one credential method per provider | UI + CLI automation, no accidental provider switch |
| API-key secrets | config.json, native masked entry; opt-in Pi read | separate auth.json, environment, references | cross-process update locks, atomic save/reload, Windows ACL, redact and logout |
| OpenAI Codex OAuth | existing native device flow and refresh | OAuth with token refresh | login → run → expiry → refresh → logout in mocked endpoint |
| Claude subscription | missing | OAuth provider-specific flow | official endpoint contract tests, no stale grants or unsafe token borrowing |
| Copilot / Gemini CLI / Antigravity | missing | provider-specific OAuth | one donor-pinned adapter at a time, refresh/cancel/race tests |
| Models | seven endpoint presets | Pi model registry and provider-specific transports | catalog/protocol parity checks, no models falsely offered |
| Additional API-key providers | not native | DeepSeek, xAI, Google, etc. | individually prove endpoint, auth, model formats and streaming |
| Errors | CLI-specific and provider-neutral error | provider resolution and refresh failure semantics | fail-closed errors, no secrets in logs/telemetry |

**Packages from Pi:** At snapshot `8b0caef...`, `@earendil-works/pi-ai` 1.4.0 is an ESM/TypeScript npm package requiring Node >=22.19.0. It cannot be linked as a Rust crate. A **separate optional Node OAuth broker** using this exact version might reduce provider login maintenance but would introduce a Node runtime, IPC, token-handling and supply-chain surface. Prefer native Rust adapters using Pi's provider implementations as an audited specification. Only select a broker after paired tests show total compatibility and advantage, and document how tokens never leak to process arguments, shell history, logs or untrusted files.

## Test discipline and release policy

This PR must pass `cargo fmt --all -- --check`, `cargo check -p finecore --locked`, and `cargo test -p finecore --locked`. The isolated tests cover JSON fixtures, provider matching, malformed input, OAuth refusal and no shell execution. No tests require or receive real provider credentials. They **do not prove** full login compatibility, OAuth refresh, success with a paid provider, or provider quality.

No merges, no main changes, no automatic credential import, no new API key secrets in Actions, and no inherited CI gates modified. Keep Draft. Source license attribution to the Pi donor is required whenever future work actually copies upstream code.
