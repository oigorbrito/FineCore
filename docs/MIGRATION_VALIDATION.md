# FineCore migration validation record

## Revisions and scope

- Source: `okikorg/orca-harness` commit `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`, tree `9c35d0dd0d7e0f226063f16dd80161e231628e2b`.
- Verbatim import commit: `e036dd7a7e2a040b4da1152d99aede0552cb1641`.
- FineCore migration worktree: branch `migration/orca-faithful-import`; working changes are not committed.
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

## Public identity integration candidate — PR #160

- At review start, remote PR head was `7ee9d3ec5255ee6120246d4154b7c242b916dd0f`, based on `migration/orca-faithful-import@53d8f8344f3f17f28766c7c880981147d3e128c6`; four commits ahead and 14 changed paths. The final correction is recorded in the PR description and its next commit.
- Reviewed all four commits and the 14-file diff. The three identity hunks from #130 were reimplemented on staging; no #130 history, Pi login, OAuth, provider-selection, or persistence changes are present. The OpenRouter title is external application metadata; inherited user-agent/referer and protocol headers remain unchanged.
- Directed semantic review found a material warning regression: the source said remote MCP transports are valid but unsupported by Orcacode v1. FineCore still supports only stdio for Agent Plugins 1.0, as stated in the existing plugin documentation. The revised FineCore warning retains that scope explicitly; no transport support or protocol behavior changed. Cross-platform Agent Plugin assertions cover manifest/client-extension warnings and the scoped MCP warning.
- The welcome-card test checks the rendered left edge against the geometric center at widths 60 and 90. This measures the rendered card rather than reading implementation internals; the focused test passes at both sizes.
- The source was assembled in an isolated validation directory from an archive of exact staging base `53d8f8344f3f17f28766c7c880981147d3e128c6` plus the remote PR code patch. Before correction, all 11 changed code-file blobs matched GitHub's PR file API. Shell fetch was unavailable because DNS could not resolve github.com; GitHub connector supplied the immutable base/head content. No pre-existing worktree was edited.
- Environment: Windows 10, Rust/Cargo 1.99.1. Commands were run offline in the isolated source directory; Cargo reused the existing local target cache.
  - `cargo fmt --all -- --check`: PASS after correcting the welcome-card formatting.
  - `cargo test --offline -p finecore --bin finecore welcome -- --test-threads=1`: 5 passed, including centering at both widths.
  - `cargo test --offline -p finecore --bin finecore model_picker_uses_the_shared_tabular_window -- --test-threads=1`: 1 passed.
  - `cargo test --offline -p finecore --bin finecore bare_prompt_is_small -- --test-threads=1`: 1 passed.
  - `cargo test --offline -p finecore --bin finecore system_prompt_instructs_concurrent_batching -- --test-threads=1`: 1 passed.
  - `cargo test --offline -p orca-harness-tool-extensions --test agent_plugin agent_plugin_warns_for_ignored_manifest_fields_and_extensions -- --test-threads=1`: 1 passed.
  - `cargo test --offline -p orca-harness-tool-extensions --test agent_plugin -- --test-threads=1`: 20 passed, 1 failed. The failure is `review_regressions::agent_plugin_cwd_expands_all_placeholders_and_accepts_repeated_separators`; the same test failed on exact staging-base source with the same Windows path-expansion signature. This inherited gate remains RED, not waived.
  - CLI MCP warning test is `cfg(unix)` and was not run on Windows. No OpenRouter network call was made; request-builder inspection confirms only the title changed.
- CI diagnosis: `.github/workflows/ci.yml` and `bench.yml` filter `pull_request` to base branch `main`; PR #160 targets staging, so those automatic triggers do not match. `ci.yml` is absent from default-branch `main`, so its `workflow_dispatch` is not an available manual trigger; GitHub documents that dispatch only receives events when the workflow file is on the default branch. No runs/status checks are present for this head. Release and paid-model benchmark workflows were not run; no workflow was modified.
- Broad workspace/Clippy/benchmark gates remain unverified or RED as recorded earlier. No merge, promotion, release or main/bootstrap mutation occurred. Migration remains `MIGRATION_PENDING`.

