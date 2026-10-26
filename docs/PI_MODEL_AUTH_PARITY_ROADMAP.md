# Roadmap: FineCore model selection and authentication parity with Pi

## Status and ordering

**Deferred.** Do not implement this roadmap until the pinned Orca-to-FineCore migration is complete and audited. This document records a later feature project; it does not expand the current migration scope.

## Reference and observed baseline

- Pi reference: `earendil-works/pi`, fetched local ref `pi-upstream/main`, commit `38ff99177fba21e9c57a670f4e95adcf407c99c7`.
- Relevant Pi implementation surfaces include `packages/coding-agent/src/modes/interactive/components/model-selector.ts`, `oauth-selector.ts`, `login-dialog.ts`, `core/auth-storage.ts`, `core/model-runtime.ts`, and model/provider registries under `packages/ai/src/`.
- FineCore migration baseline: Orca Harness source commit `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`.
- FineCore currently has a fixed seven-provider enum in `crates/cli/src/msg.rs`, model/provider selection in the Ratatui TUI, and a Codex device-login flow under `crates/cli/src/auth/`. A live authenticated login has not been verified.

Pi is TypeScript and FineCore is Rust; the target is **observable functional parity**, not copying Pi's implementation or UI toolkit. Provider coverage is not assumed to be identical until an explicit compatibility inventory is approved.

## Deferred project stages

### 0. Contract and provider inventory

- Pin the Pi revision and inventory its selector behaviors, model catalog sources, supported credential types, login/logout flows, and persistence/security guarantees.
- Inventory FineCore providers, catalog behavior, persisted settings, credential locations, and compatibility obligations.
- Decide which Pi providers/auth methods have viable Rust backends and whether unsupported providers should be hidden or reported as unsupported.
- Define observable acceptance tests before implementation. No credential migration or deletion without explicit approval.

### 1. Provider and model registry

- Replace hard-coded selection assumptions with a provider/model registry that can represent configured, unavailable, and dynamically discovered models.
- Preserve existing FineCore provider IDs, configuration keys, and session compatibility; define aliases/migrations separately if names must change.
- Support cached catalogs, refresh, partial provider failures, cancellation, and deterministic ordering without losing a user's current selection.

### 2. Model-selector behavior

- Search/filter models across configured providers; display provider, model ID/name, current selection, and default selection.
- Provide clear empty, loading, stale-cache, refresh-error, and no-configured-provider states.
- Support selecting the active model and saving a default model, with persistence and selection restoration covered by tests.
- Match Pi interaction semantics where appropriate while following FineCore's Ratatui keybinding conventions.

### 3. Login/logout and credential management

- Add provider-driven `/login` and `/logout` flows with searchable providers, auth type/status, configured source, and clear cancellation/error states.
- Define a credential interface for API keys and OAuth/device/browser flows; keep secrets out of logs, UI output, tests, and telemetry.
- Implement atomic storage, permission protections, refresh/expiry behavior, and explicit behavior when multiple processes modify credentials.
- Keep environment variables and existing FineCore/Orca credential files compatible; do not silently import, overwrite, or delete credentials.

### 4. Verification and rollout

- Unit-test search, ranking, default/current selection, catalog refresh/error/cancellation, credential-source precedence, OAuth state validation, token refresh, persistence, and logout.
- Add mocked provider protocol tests; live login tests remain opt-in and require explicit credentials/approval.
- Validate API-key redaction, file permissions, URL/callback validation, CSRF/state handling, and failure cleanup.
- Compare supported-provider and model-catalog behavior against the pinned Pi contract. Publish an explicit parity matrix; do not claim complete parity where providers are unsupported.

## Acceptance criteria

- Approved provider/auth parity matrix implemented or each gap clearly marked.
- Model selector and login/logout flows have deterministic, automated coverage including errors and cancellation.
- Existing FineCore configuration and credentials remain usable without destructive migration.
- No live credentials or network login is required for standard CI.
- Security review and compatibility review completed before release.
