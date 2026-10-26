# FineCore — auditable integration strategy (Orca migration)

**Status:** REVIEW CANDIDATE ONLY. No PR or merge to `main` is authorized by this record.

## Decision and immutable inputs

- `main` base: `95e666924e672dba4749ab02b26d86782b0bf78a`.
- Pinned donor: `okikorg/orca-harness@2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`, source tree `9c35d0dd0d7e0f226063f16dd80161e231628e2b`.
- Accepted migration snapshot: `migration/orca-faithful-import@2d95a81054734730715369d54f64b28f419734fc`, tree `19aa68ed3c88ab8c9a3acbdc2be0a65d32ea4195`.
- Those branches have **no common Git ancestor**. GitHub's normal compare endpoint returns HTTP 404.
- The integration branch starts from the pinned `main` and imports the *content*, not a synthetic/forced history merge. The original migration branch and its commits remain unmodified and separately traceable.

## Transport construction and verification

1. First review commit stages all 1,101 original donor file blobs *and their Git executable modes* at repository root, preserving the 25 paths exclusive to `main`. Its tree is `59ffaea83654ad0d76c904ea87ce5388da512dda`.
2. Second review commit applies exactly 61 modified source paths and 16 additional paths from the verified migration head. All 1,117 migration file blobs/modes must match the migration tree exactly. It retains all 25 main-only files, for **1,142 tracked files** before integration-only controls.
3. The only direct name collision is the root `README.md`. The FineCore README takes precedence; the template remains at `evidence-repo-template/README.md`.
4. The main-only upstream-tracker workflow, its Python implementation and tests, and its documentation must remain unchanged.
5. A new offline verifier checks the two manifests against the Git objects, confirms `main` ancestry, and checks those 25 retained files. This is *not* equivalent to application runtime acceptance.

No `git merge --allow-unrelated-histories`, rebase, force push, or history rewrite is used. This is a content-transplant commit chain whose parent is `main`; the original donor and migration commit IDs remain recorded as immutable provenance.

## Evidence and known inherited failures

- `docs/upstream-import-manifest.csv`: all 1,101 donor paths identical at the original import.
- `docs/identity-divergence.csv`: 1,040 identical, 61 modified and 15 added paths; it intentionally excludes itself, the sixteenth added file.
- `docs/MIGRATION_VALIDATION.md` and `docs/validation-logs/`: differential Linux/WSL ext4 checks with Rust 1.99.1 and Bun 1.4.4. CLI test-name delta is explained by updater policy; upstream and FineCore share six Bun REPL failures, 13 strict Clippy diagnostics and 15 source-size violations. Both exceed the 6,700,000-byte Linux binary-size limit. These are **non-green gates**, not waived passing checks.
- Any CI on this branch should still report actual errors. An inherited failure is not a license to ignore new failures.

## Integration review gates

- [ ] Confirm the exact source/migration/manifests using `python3 scripts/verify_finecore_integration.py`.
- [ ] Execute Linux/macOS integration checks against the **new combined tree**, independently of the historical WSL comparison; review failures against source and migration evidence.
- [ ] Audit `.github/workflows/ci.yml`, `bench.yml`, `bench-live.yml`, and `release-orcacode.yml`: default-branch CI and benchmarks are inherited; paid model benchmark and Orcacode release jobs must remain guarded to `okikorg/orca-harness` only. The existing upstream tracker must keep its independently proven schedule and review-only behavior.
- [ ] Reconcile *dated statements* in `docs/MIGRATION_VALIDATION.md` (notably its pre-commit worktree status) and the `npm/finecore` reservation placeholder. The operator separately reported publishing an actual functional `finecore@0.0.1` bootstrap under npm account `engfinecode`; do not publish the stale placeholder package or confuse it with the Rust CLI. Verify the registry record when updating release documentation.
- [ ] Review security/license/attribution, CI cost, branch protection, and repository identity after the combined tree is checked.
- [ ] Open a review PR only after the above scope is understood. **Manual merge only on explicit authorization**; do not auto-merge or change `main` as part of this preparation.

## What this does not establish

Successful source transport and differential parity do **not** establish that FineCore is faster, more economical, or more accurate than Pi. Those outcomes require separate matched-task performance/quality experiments and real task outcomes. Historical Orcacode benchmark files are *upstream evidence*, not FineCore measurements.
