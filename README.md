# FineCore

Independent clean-room starting point. This repository does not include donor source or donor Git history.

## Scope of the initial kernel

A dependency-free Rust execution state machine with explicit transitions, terminal states, and unit tests. No claim of parity with any existing agent harness.

## Validation

`cargo test --all-targets` and `cargo fmt --all -- --check` (requires Rust toolchain). These commands have **not** been executed in the packaging environment.

## Git history

This directory is initialized with a single root commit with zero parents. The `main` branch of the remote FineCore repository is **not** modified.
