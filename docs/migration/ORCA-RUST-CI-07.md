# ORCA-07 — Rust validation gates (execution pending)

Source: `vendor/orca-harness/`, pinned tree `9c35d0dd0d7e0f226063f16dd80161e231628e2b`.
This is a reproducible execution plan, **not** a claim that Cargo passed.

## Independent gates

1. **Provenance:** `git rev-parse HEAD:vendor/orca-harness` equals pinned tree; worktree clean under `vendor/orca-harness`.
2. **Toolchain:** record `rustc --version --verbose`, `cargo --version`, platform, lockfile checksum. Prefer the donor's documented/pinned toolchain when present; do not silently change dependencies.
3. **Formatting:** `cargo fmt --all -- --check`.
4. **Compilation:** `cargo check --workspace --locked`.
5. **Tests:** `cargo test --workspace --locked`.
6. **Lint:** `cargo clippy --workspace --all-targets --locked -- -D warnings` (if upstream baseline contains warnings, document exact baseline and separate regression from inherited debt).

Run from `vendor/orca-harness` with isolated `CARGO_TARGET_DIR` and `CARGO_HOME` if feasible. Retain complete logs, exit codes, runner metadata, hashes and duration per gate. No model API calls, benchmark claims, releases, package publishing, auto-merge, or changes to upstream source. Do not infer performance from Cargo test timings.

## Windows and Linux

Use Linux CI first for repeatability. Windows remains an independent portability gate and must not be marked passed by Linux results. If a dependency requires secrets/network, classify as environment blocker rather than a code regression without evidence.

## Required outcome classification

`PASS`: command executed with exit 0 and attached log.
`FAIL`: executed, nonzero, reproducible code/test failure.
`BLOCKED`: no runner, missing toolchain/credentials, network or permissions.
`NOT_RUN`: command not executed.

At creation, all Rust gates are **NOT_RUN**. GitHub workflow creation is restricted in the current connector session; this branch contains documentation only. Separate ORCA-06 scanner and unit tests remain on PR #4, and are not imported here.
