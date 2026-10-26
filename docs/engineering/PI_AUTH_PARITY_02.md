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

The runtime API-key resolution order is the explicit command-line key, then the opt-in Pi compatibility file, then the existing FineCore environment-variable/config resolution. This intentionally differs from Pi's original credential precedence; FineCore's existing environment-over-config rule is preserved. After `/login`, the UI states whether the saved FineCore key is effective or whether the configured Pi entry/environment variable takes precedence. The `/provider` key flow says that the entered key is used for the current session and reports the resolution order for future sessions. The Pi file is never read unless explicitly configured, and no Pi file is modified.

New API keys use FineCore's existing config store, not a new token database. On Unix, the existing writer creates/restricts the config file to owner-only mode (`0600`). On Windows, the writer now applies a protected DACL granting full control only to the file owner and SYSTEM before writing config content; `config_file_has_a_protected_owner_only_acl` checks the resulting ACL with `icacls`. This is access-control hardening, not encryption or an OS keychain. It does not protect against code running as the same Windows user. Codex-owned OAuth data uses a separate store and is not covered by this config-file ACL change; that store needs its own Windows security review before claiming secure persistence for OAuth tokens.

Keys are masked in the TUI while entered; rendered provider/logout rows expose status only, not credential contents. Errors and confirmations identify the provider and config path but do not include key material. Provider tests use local test values and perform no network authentication.

## Windows comparison

The full FineCore CLI suite was compared sequentially on one GitHub-hosted `windows-2025` runner with Rust/Cargo 1.99.1 and separate checkouts/targets: PR #120 head `06226adecce1671205607c8c79e42dbcb339c05c` versus PR #121 head `a98ad059e776ccb7723445324c1a980091f82a32`. Baseline: 620 passed, 10 failed, 1 ignored. Candidate: 628 passed, 10 failed, 1 ignored. All ten failing test names are identical; neither side had unique failures. The comparison run is red because both full suites exit 101, and the workflow intentionally preserves that failure rather than masking it. The candidate's eight additional passing tests include the Windows ACL check. Summary evidence is in `pi-auth-parity-02/evidence/windows-comparison-40415410811.json`; [run 40415410811](https://github.com/oigorbrito/finecore/actions/runs/40415410811) preserves raw logs in artifact `12695912983`. This supports classifying the ten Windows failures as already present at PR #120's tested head, not as regressions in the compared PR #121 head. It does not establish that either suite is Windows-clean.

## Scope and remaining gaps

This is a first interactive manager, not the complete Pi login experience. FineCore currently supports API-key entry for providers in its existing catalog and its existing native OpenAI Codex OAuth. It does not add Pi's Claude, GitHub Copilot, Google, or other provider-specific OAuth flows, device flows, refresh-token management, provider catalog, or provider/model parity. Logout does not revoke external credentials. Pi-auth import remains opt-in and read-only.

No claims are made about full Pi parity. Keep the implementation Draft until provider-specific flows, secure persistence on each supported OS, and user-facing requirements have independent review.

## Validation

- `cargo fmt --all -- --check`
- `cargo check -p finecore --locked`
- Focused auth, login/logout UI, and Windows config-DACL tests passed locally.
- Linux Actions run [40415410713](https://github.com/oigorbrito/finecore/actions/runs/40415410713) passed formatting, comparison-script tests, compilation, auth/UI tests, and the full CLI regression suite.
- The paired Windows comparison above is intentionally failing because both revisions retain the same ten Windows failures; see its evidence artifact.

The login/logout tests exercise local storage and UI command routing only. They do not invoke a model, provider, or OAuth endpoint.
