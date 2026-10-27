# PI_AUTH_CODEX_INTERACTION_02 — FineCore CLI integration

**State:** local follow-up branch `feature/pi-auth-codex-login-interaction-02`, based on Draft PR #122 head `7dd017b1782f4c18004c3d5f7c6d58cb17804fd5`. Not pushed or opened as a PR. No merge or release.

## Scope

This follow-up connects PR #122's Rust browser protocol primitives to FineCore's existing `/login` command. It does not import or launch Pi's TUI, add Node, replace the Rust model engine, or change the command name.

- `/login openai-codex` and the login picker now offer browser login and the existing device-code login. Device code continues through the existing `DeviceOAuth` implementation.
- Browser login displays the complete authorization URL only inside an ephemeral FineCore login overlay. It is not added to transcript history.
- The overlay accepts pasted code or redirect URL, hides the input, and submits through a one-shot channel. Callback, manual input, and Escape cancellation are connected to the same login attempt; a completed attempt closes the overlay.
- If port 1455 cannot be bound, browser login remains available through manual input.
- Successful login writes to FineCore's existing Codex-owned file; it does not alter Pi's `auth.json` or Codex CLI's credential file. Existing unknown fields in FineCore's JSON are retained during login and refresh. A malformed existing file fails closed rather than being overwritten.
- If login completes while Codex is already active, FineCore rebuilds its local runtime auth source without switching providers or fetching a model catalog.

This is an interaction integration, **not full Pi credential-store parity**. Persistence remains in FineCore's existing location and schema. Cross-process locking and the Windows ACL audit remain separate gates.

## Reproducible checks

Local Windows commands:

```sh
cargo fmt --all -- --check
cargo check -p finecore --locked
cargo test -p finecore codex --locked
cargo test -p finecore --locked
```

Results observed for this working tree:

- Formatting: PASS.
- `cargo check -p finecore --locked`: PASS.
- `cargo test -p finecore codex --locked`: 20 passed, 0 failed. Browser tests use loopback/mock OAuth responses; no OpenAI or model API call.
- Full Windows CLI suite (latest run): **FAIL**, 640 passed, 10 failed, 1 ignored. All ten failed test names match the previously recorded PR #120/#121 Windows failure set; no failure was unique to this follow-up. One earlier full run also failed `streaming_frame_deadline_flushes_without_another_event`, which passed in isolation and in the latest full run, indicating timing sensitivity. The suite remains red and is not claimed clean.
- Existing warnings in clipboard-image and MCP plugin code remain; no lint suppression or warning changes were made.

No Linux run has been performed for this follow-up. PR #122's earlier Linux/Windows protocol checks are evidence for its base only, not for these CLI/UI changes.
The latest full-suite output is preserved locally at `D:/temp/finecore-pi-auth-interaction-02-windows-test.log`; it is not committed as a repository artifact.

## Remaining gates

1. Obtain paired Linux/Windows CI for this follow-up and compare the Windows failure set with the same pinned baseline.
2. Review cross-process login/refresh serialization and Windows permissions for FineCore-owned OAuth data before claiming secure or complete persistence parity.
3. Keep Draft; do not test real provider login until explicitly authorized.
