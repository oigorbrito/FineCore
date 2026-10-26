# PI_AUTH_CODEX_BROWSER_01 — native browser protocol parity, not yet TUI activation

**Date:** 2026-10-26. **State:** experimental Draft stacked on FineCore PR #121; never merge as a claim of full Pi parity. The user's local `feature/pi-auth-codex-native-01` was not present on GitHub when checked. This work is on a separate remote branch so it cannot overwrite that local work.

## Source of truth

- [Pi canonical Codex OAuth](https://github.com/earendil-works/pi/blob/main/packages/ai/src/auth/oauth/openai-codex.ts), audited file SHA `860ea75542fe05b2f10df2460d89ff89c5e28997`
- FineCode's identical Codex OAuth file SHA (confirmed in repository comparisons): `860ea75542fe05b2f10df2460d89ff89c5e28997`
- Rust FineCore base is PR #121 HEAD `5b652a84f413a08da11305f937af7df0c7e30877`.
- **Do not use Pi's distinct `openai-chatgpt.ts` dynamic-client OAuth protocol as if it were Codex OAuth.** They use different authorization endpoints, scopes, exchange and credential contracts.

## Behavior implemented in this bounded primitive

1. Generate a 32-byte cryptographically random PKCE verifier, S256 challenge and independent 16-byte state, matching the Pi Codex protocol parameters: authorize endpoint, callback URL `http://localhost:1455/auth/callback`, scope, client ID and client flags. Keep the FineCore brand as `originator`.
2. Bind a loopback-only HTTP listener. Reject wrong method, host, route, missing state, mismatched state and missing code. Return no tokens, user codes or secret material in the browser response.
3. Parse Pi's manual fallback forms: code, code#state, query parameters, and full redirect URL. An explicitly present incorrect state fails closed.
4. Accept either a browser callback or a manual oneshot channel. If port 1455 is occupied, calling code can pass `None` for callback and still use manual entry.
5. Exchange the authorization code using the same authorization-code + PKCE form parameters. Validate a synthetic token, derive ChatGPT account ID, and return token material to the established Codex persistence path. The test does not call OpenAI or another external provider.
6. Leave the existing native device authorization path, rotating refresh logic, storage path and owner of imported official Codex credentials untouched.

## Deliberate non-claims

**No TUI integration has been performed in this PR yet.** The Rust primitives are not invoked by the existing `/login openai-codex` handler, which still uses device code. Wiring a Pi-like browser/device-method selection and manual-code input UI without blocking the normal terminal is the next integration gate. The browser protocol implementation therefore must not be described as end-user login parity.

No migration, deletion or rewrite of stored credentials; no Pi npm package is installed, because this bounded protocol implementation is compiled into the Rust CLI. Pi-like persistence semantics and cross-process refresh/storage locking remain a separate gate; passing Rust unit tests is not proof of full equivalence.

## Tests / promotion gates

The isolated GitHub Actions workflow compiles with `--locked` on Ubuntu and Windows, then exercises real localhost sockets (not just mocked methods), wrong-state/wrong-host/wrong-path denial, manual-code arbitration, synthetic authorization-code HTTP exchange, a synthetic 401 with secrets redacted, and existing device-code regressions. A full Linux CLI run acts as regression evidence. **No provider or model calls are made.** Tests are not a substitute for final protocol compatibility verification against a real provider after user authorization.

Before marking this Draft ready: implement Pi-style login method choice, browser auth URL display and manual fallback in FineCore TUI; confirm cancellation/timeout/port-collision behavior, prompt secret handling and Codex-owned token preservation. On Windows, require secure ACL verification for FineCore-owned OAuth storage and do not modify any external Codex CLI file's ACL. This avoids conflating a Pi-compatible browser handshake with full credential storage parity.

No merges, main changes, provider calls or automatic PR promotions.
