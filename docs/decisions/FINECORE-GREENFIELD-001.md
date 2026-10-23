# FineCore — greenfield reset (decision record)

Status: proposed, **not** a clean-history implementation branch.

## Binding product decision

FineCore is a new, independently implemented project. No Orca/Pi source-tree import, no inherited Git ancestry in the eventual canonical root, no Orca-vs-Pi benchmark. Previous donor migration branches and PRs are superseded as implementation candidates; do not merge them.

## Important Git distinction

A branch created from existing `main` **inherits its ancestry**. This branch is only a temporary decision record. It must not be promoted as a clean-history root. A clean root requires an orphan commit (or a new repository), verified with `git rev-list --parents -n 1 HEAD` returning only the commit SHA, and a file-tree audit excluding donor code.

## First independent implementation slice

Implement from original requirements, not copied donor internals:
- Rust workspace: core library and CLI.
- Versioned newline-delimited JSON event envelope.
- Explicit execution states and typed error reporting.
- Minimal CLI with `--version`, `--help`, and offline `--json` diagnostic.
- Deterministic unit/integration tests; no network or model keys required.

Acceptance: original source files; a root commit with zero parents; Cargo format/check/test pass on a recorded toolchain; provenance and dependency license audit; no automatic merge. Do not represent an unrun gate as passed.

## Existing PRs

Migration PRs #3, #4 and #5 remain separate and should be reviewed for closure, not merged into the new canonical history. No destructive changes to `main` without explicit authorization.
