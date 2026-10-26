# FineCore — canonical `main` integration (Orca migration)

**Status:** integration candidate on `integration/finecore-orca-main-review`; `main` remains unchanged. No PR or merge has been performed in this review block. The intended route is a normal review PR to `main`, never a forced history rewrite.

## Immutable inputs and lineage

- Current remote `main`: `95e666924e672dba4749ab02b26d86782b0bf78a`.
- Pinned donor: `okikorg/orca-harness@2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`, source tree `9c35d0dd0d7e0f226063f16dd80161e231628e2b`.
- Accepted migration snapshot: `migration/orca-faithful-import@2d95a81054734730715369d54f64b28f419734fc`, tree `19aa68ed3c88ab8c9a3acbdc2be0a65d32ea4195`.
- The original migration branch and remote `main` have no common ancestor. The integration candidate is a content transplant whose first parent is `main`; it preserves `main` ancestry and permits an ordinary PR/merge review.

## Contents and preservation contract

1. Commit `25bb2a705be2f6903c52b3a2507159b9d98b4a89` stages all 1,101 original donor file blobs and executable modes. It preserves the 25 paths exclusive to `main`; its tree is `59ffaea83654ad0d76c904ea87ce5388da512dda`.
2. Commit `0e6fc6dc761b2a03ffcc180b2691c00bcfa4268b` applies the 61 classified source changes and all 16 additional paths from the migration snapshot. It retains the 25 main-only paths. The migration snapshot contributes 1,117 paths; together with main-only paths this is 1,142 before integration controls.
3. Commit `cb143d3b343eb91c81c809ac71b221b3c20959ef` adds the read-only verifier, CI workflow and integration record. The original migration branch and its commit IDs remain unchanged.
4. The source import/divergence manifests remain the accepted migration snapshot. `docs/integration-divergence.csv` separately records five review-layer corrections to README performance attribution, validation/package status, identity inventory, and the internal npm placeholder. The verifier checks these overlay blobs against the combined tree.
5. The only original root-name collision is `README.md`; the FineCore README takes precedence. The upstream-tracker workflow, its tests, docs and other 24 main-only paths remain unchanged. The offline verifier checks all 25 main-only paths by Git blob.

No `git merge --allow-unrelated-histories`, rebase, force push or rewrite of `main` is used. The complete upstream import remains available on `migration/orca-faithful-import`; this branch layers its verified content onto the existing `main` ancestry.

## Evidence

- `scripts/verify_finecore_integration.py` verifies donor source blobs, the accepted migration manifests, the integration overlay, main ancestry and retained main-only content.
- The corrected source audit passed locally at `211bc16` (1,101 donor paths; 1,040 identical, 61 modified, 15 classified additions; five recorded overlay corrections; all 25 main-only paths preserved; 1,146 combined paths). GitHub run [`40260695414`](https://github.com/oigorbrito/finecore/actions/runs/40260695414) passed on that head for Linux and macOS: tracker tests, formatting, workspace `cargo check --all-targets`, and FineCore CLI tests. Later branch changes are limited to this integration record; no source or test code changed after that run.
- The pinned-source differential record and raw logs are in `docs/MIGRATION_VALIDATION.md` and `docs/validation-logs/`. They show the CLI test-set change is intentional; six Bun failures, 13 strict Clippy errors and 15 source-size violations match the pinned source. Both Linux release binaries exceed the inherited size limit. These are red gates, not waived passes. The integration workflow does not claim full-workspace tests or Clippy are green.
- `README.md` now labels the historical Orcacode performance section as upstream evidence, not FineCore measurements. No FineCore performance benchmark was performed.
- Read-only npm registry metadata on 2026-10-26 confirms a separate public `finecore@0.0.1` bootstrap package for Rust toolchain diagnostics/workspace inspection. `npm/finecore/` is an internal placeholder, marked `private: true`; it is not the published package or the Rust CLI and must not be published.
- Release and paid live-benchmark jobs remain guarded to `okikorg/orca-harness`; the independently tested upstream tracker remains intact. No external paid calls were made.

## Remaining gates before a review PR

- [x] Re-run `python3 scripts/verify_finecore_integration.py` after the overlay corrections.
- [x] Obtain a successful read-only Linux/macOS integration workflow run for the corrected source/test head.
- [x] Review the integration overlay, source attribution, identity corrections, retained main-only paths, and known inherited red workspace gates.
- [ ] Open a **Draft PR** from `integration/finecore-orca-main-review` to `main` for review. This does not merge or change `main`.

Any merge to `main` remains manual and requires separate explicit authorization. This transport does not establish FineCore performance or live legal-quality claims.
