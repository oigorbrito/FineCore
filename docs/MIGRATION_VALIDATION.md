# FineCore migration validation record

## Revisions and scope

- Source: `okikorg/orca-harness` commit `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`, tree `9c35d0dd0d7e0f226063f16dd80161e231628e2b`.
- Verbatim import commit: `e036dd7a7e2a040b4da1152d99aede0552cb1641`.
- Validation workspace: branch `migration/orca-faithful-import`; at the time these checks ran, working changes were uncommitted. This record does not establish the current branch or worktree cleanliness.
- Source/destination inventory: 1,101 source files and 1,101 destination files at the import commit; all 1,101 blobs compare identical. See [`upstream-import-manifest.csv`](upstream-import-manifest.csv).
- Post-import comparison: 1,040 source blobs remain identical; 61 source files have classified changes, with no missing or unexplained source paths; 15 FineCore integration/evidence files are recorded as added (including eight raw Linux gate logs, an unpublished npm name-reservation placeholder, and a benchmark-report regression test). See [`identity-divergence.csv`](identity-divergence.csv); that report excludes itself. Changes cover FineCore metadata/CLI/docs and generated plugin scaffolds, CI and benchmark invocation plumbing, and fail-closed upstream release/update paths. Public `orca-harness-*` crates, provider/config IDs, persistent paths, and upstream attribution/notices remain unchanged.

## Environment

- Initial comparison: Windows 10 x86_64 under MSYS (`MINGW64_NT-10.0-26300`).
- Supported-platform pass: WSL2 Ubuntu x86_64, both snapshots under `/home/igorb` on Linux ext4 (CRLF normalized as a checkout would be), `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1 (5f94df478 2026-08-27)`, native Bun 1.4.4. The pinned source and FineCore snapshot were run with the same toolchain/runtime to distinguish inherited failures from migration regressions.
- Raw logs for the differential CLI, workspace/Bun, Clippy, and source-size checks are retained in [`validation-logs/`](validation-logs/).

## Commands and results

| Command | Pinned source | FineCore | Result / interpretation |
|---|---|---|---|
| `cargo fmt --all -- --check` | PASS | PASS | Formatting gate passes. |
| `cargo build -p finecore` | n/a | PASS | FineCore CLI builds. |
| `cargo test -p orcacode` / `cargo test -p finecore` | 616 passed, 10 failed, 1 ignored | 614 passed, 11 failed, 1 ignored | Ten failures match exactly: Windows path separators/permissions and existing shell-policy, plugin/config, and changelog expectations. One theme-picker test failed only in the full FineCore run, then passed individually on both trees; classify as flaky/order-dependent, not migration-introduced. FineCore removes two passing upstream updater tests and adds one fail-closed test, explaining the net one fewer pass. This is not a passing test gate. |
| `cargo test -p orcacode --test plugin_cli` / `cargo test -p finecore --test plugin_cli` | 4 passed, 2 failed | 4 passed, 2 failed | The same two plugin integration tests fail on both revisions due to Windows path/script behavior and uninstall output. |
| `cargo test --workspace` (Windows/MSYS) | FAIL | FAIL | Both fail compiling `crates/sandbox-providers/src/local/protect.rs:142`, which calls `std::os::unix::fs::symlink` on Windows. |
| `cargo test -p orcacode` / `cargo test -p finecore` (WSL2 ext4) | 650 passed, 0 failed, 1 ignored | 649 passed, 0 failed, 1 ignored | The only test-name difference is intentional updater policy: Orca-only `update::tests::update_comparison_is_numeric_and_fails_closed` and `update::tests::updater_uses_the_public_installer_and_propagates_download_failures`; FineCore-only `update::tests::update_fails_closed_without_a_finecore_release_channel`. The other 654 test names match; both six-test `plugin_cli` suites pass. Full output: [`linux-cli-upstream.log`](validation-logs/linux-cli-upstream.log), [`linux-cli-finecore.log`](validation-logs/linux-cli-finecore.log). |
| `cargo test --workspace` (WSL2 ext4, Bun 1.4.4) | FAIL (6 `bun_repl` tests) | FAIL (same 6 tests) | The same six test names, panic locations, and expected/actual output reproduce on both trees; other workspace tests pass. Logs: [`linux-workspace-upstream.log`](validation-logs/linux-workspace-upstream.log), [`linux-workspace-finecore.log`](validation-logs/linux-workspace-finecore.log). |
| `cargo clippy --workspace --all-targets -- -D warnings` (WSL2 ext4) | FAIL (13 `result_large_err` errors) | FAIL (same errors) | The identical 13 locations occur in `harness-core`: `agent.rs:68,79,94`; `agent_loop.rs:29,42,106`; `dispatcher.rs:83,379,499`; `extension.rs:322,333,344`; `testing.rs:70`. Logs: [`linux-clippy-upstream.log`](validation-logs/linux-clippy-upstream.log), [`linux-clippy-finecore.log`](validation-logs/linux-clippy-finecore.log). |
| `bash ci/check-source-size.sh` (WSL2 ext4) | FAIL (15 files >599 lines) | FAIL (same 15 files) | The exact same 15 paths and line counts occur in both snapshots; no files were split as part of identity work. Logs: [`linux-source-size-upstream.log`](validation-logs/linux-source-size-upstream.log), [`linux-source-size-finecore.log`](validation-logs/linux-source-size-finecore.log). |
| `bash benchmarks/run_reporting_tests.sh` (Windows/MSYS) | FAIL (2 Windows path cases) | FAIL (same 2 cases) | `harness-comparison` expects POSIX separators; binary-size test classifies Windows path separators as a non-musl target. |
| `bash benchmarks/run_reporting_tests.sh` (WSL2 Linux) | PASS | PASS | The complete reporting suite passes on Linux; one live-run budget-key check skips because there is no recorded live run. |
| `cargo clippy -p orcacode --all-targets` / `cargo clippy -p finecore --all-targets` | PASS with warnings | PASS with warnings | FineCore adds no observed Clippy errors; existing warnings remain. |
| `cargo bench --workspace -- --test` (WSL2 Linux, Bun 1.4.4) | FAIL (same 6 `bun_repl` tests) | FAIL (same 6 tests) | Benchmark smoke runs the affected package tests; this reproduces the upstream Bun failures, not a FineCore-only regression. |
| `cargo build --release -p orcacode` / `cargo build --release -p finecore` (WSL2 Linux) | PASS | PASS | Both release binaries build successfully. |
| `ci/check-binary-size.sh target/release/{orcacode,finecore}` (WSL2 Linux) | FAIL (9,347,152 bytes) | FAIL (9,338,608 bytes) | Both exceed the inherited 6,700,000-byte GNU/Linux limit. FineCore is 8,544 bytes (~0.09%) smaller; this size difference is not performance evidence. |
| `cargo test -p finecore --bin finecore update::tests` | n/a | PASS (1 test) | Confirms updater fails closed without contacting Orcacode's release service. |
| `cargo test -p finecore --test plugin_cli python_and_typescript_scaffolds_have_expected_layout_and_do_not_overwrite` | n/a | PASS (1 test) | Confirms generated plugin READMEs and metadata use FineCore commands/identity while preserving the compatibility namespace. |
| `python3 -m unittest discover -s benchmarks/swebench-lite -p '*_test.py' -v` | n/a | PASS (6 tests) | Confirms FineCore result identity and the retained runner compatibility alias. |
| `PYTHONPATH=benchmarks/harness-comparison python3 -m unittest run_test.RunTest.test_finecore_binary_option_and_legacy_alias -v` | n/a | PASS (1 test) | Confirms `--finecore-bin`, its FineCore default, and the compatibility alias. |
| `cargo run -p finecore -- --version` | n/a | PASS (`finecore 0.9.0`) | Confirms the built CLI presents the FineCore binary identity. |
| `PYTHONUTF8=1 python3 docs/external/check.py` | n/a | PASS (24 pages, 34 routes) | Default invocation fails on this Windows locale while reading UTF-8; UTF-8 mode passes. |
| `python3 -m unittest discover -s benchmarks/compare -p '*_test.py' -v` | n/a | PASS (1 test) | Confirms active comparison report renders FineCore labels rather than inherited Orcacode labels. |
| `bash benchmarks/run_reporting_tests.sh` | n/a | FAIL (Windows-specific paths) | Core-tools, kernel, MCP, subagent, SWE-bench-lite and workflow suites pass; one harness-comparison test and one musl-target binary-size test fail due to Windows path separators. These are platform-sensitive and require supported Unix validation. |
| `bash ci/release.sh tag 0.1.0` | n/a | FAIL CLOSED as intended | Refuses Orcacode release operations before tag or network activity; no tag/publish attempted. |

`cargo fmt --all -- --check`, both CLI-package test suites on WSL2 ext4, the complete benchmark reporting suite on Linux, 21 startup-budget tests, 6 SWE-bench adapter tests, the FineCore binary-option test, and the external-doc structural check (with `PYTHONUTF8=1`) pass. FineCore also builds in release mode. The CLI test-set delta is fully explained by the deliberate removal of the two upstream-installer tests and addition of the FineCore fail-closed test. The remaining red workspace gates—source size, strict Clippy, Bun REPL tests, and GNU/Linux binary size—reproduce on the pinned source; exact differential logs are retained above. `ci/release.sh tag 0.1.0` was exercised only to confirm its fail-closed guard; no tag or network action occurred.

No live provider login, external API call, or benchmark measurement was performed. The historical Orcacode benchmark runners/results are retained as source evidence; they are not FineCore performance measurements. A separate FineCore benchmark baseline is needed before performance claims. Benchmark reporting failures and the source-size gate were reproduced on the pinned source; they are pre-existing, not migration regressions.

## Acceptance assessment and deferred work

- No unexplained CLI test-name or failure delta remains in the tested Linux package/workspace gates. All 61 upstream-path changes are classified in [`identity-divergence.csv`](identity-divergence.csv); the gate logs above retain test names, diagnostics, Clippy locations, and source-size paths/counts for review.
- Do not fold inherited source-size, Clippy, Bun REPL, or binary-size failures into this identity migration; address them as separate engineering work if approved.
- Preserve manual review for any eventual merge to `main`; model-picker/login parity remains deferred to [`PI_MODEL_AUTH_PARITY_ROADMAP.md`](PI_MODEL_AUTH_PARITY_ROADMAP.md).
- The npm package at `npm/finecore/` remains an unpublished placeholder. Registry lookup returned 404 (available at check time), but `npm whoami` returned 401, so the name has not been reserved.


## Current staging delta after the recorded validation

The preceding commands and raw logs describe the earlier imported product snapshot. They do not validate later code changes.

- PR #135 was merged into `migration/orca-faithful-import` at `53d8f8344f3f17f28766c7c880981147d3e128c6`. It changed `Cargo.lock`, `crates/cli/Cargo.toml`, `crates/cli/src/config/storage.rs`, and `crates/cli/src/config/tests.rs`.
- At that staging revision, the GitHub blobs for `storage.rs` and `tests.rs` contain 835 and 935 lines respectively. The unchanged CI source-size gate rejects files above 599 lines. These are two additional current RED paths; the 15 earlier source-size paths are untouched by PR #135. The current tree therefore has at least 17 source-size violations.
- At the staging base `.github/workflows/ci.yml` triggers on pushes and pull requests to `main`, plus manual dispatch. The Actions API showed no workflow run for commit `53d8f834`; this is NOT RUN evidence, not a pass. This PR proposes adding push and pull request triggers for `migration/orca-faithful-import`; that change requires a post-merge staging run before it can supply current validation.
- A Windows check on the predecessor code at `c3bbf29` passed formatting, while `cargo test --workspace --no-fail-fast` failed compiling the Unix-only symlink test and strict Clippy failed with 13 `result_large_err` diagnostics in unchanged `harness-core` files. Those results do not validate PR #135 or the current `53d8f834` staging tree.
- Current full-workspace tests, strict Clippy, Linux/Windows validation of PR #135, and current release-size validation remain NOT RUN or unverified at `53d8f834`. Keep migration status `MIGRATION_PENDING` until the affected gates are rerun on the current revision and compared with the pinned baseline.
