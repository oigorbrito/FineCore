---
name: finecore-migration-identity
description: Evidence-gated migration and canonical FineCore identity across isolated Git histories; use before importing, branding, adapting, proposing, or promoting harness changes.
---

# FineCore migration and canonical identity — mandatory procedure

## Authority and scope

This skill governs changes involving the Orca-to-FineCore import, canonical product identity, compatibility aliases, upstream adaptations, documentation and benchmark provenance. Read the repository root `AGENTS.md`, `docs/FINECORE_IDENTITY_INVENTORY.md`, `docs/identity-divergence.csv`, `docs/upstream-import-manifest.csv`, and `docs/MIGRATION_VALIDATION.md` before acting. If files disagree, stop, record the discrepancy, and resolve via evidence; do not assume this skill supersedes Git reality.

**Protected histories:** remote `main` contains tracker/evidence history only, not the Rust harness. `bootstrap/finecore-independent` is greenfield. `migration/orca-faithful-import` descends from bootstrap and is the designated harness staging base. Never merge harness commits into remote `main`, rewrite unrelated histories, force-push, rebase destructively, modify branch protection, delete worktrees/aliases, publish releases, or auto-merge. Promotion to bootstrap is a separate explicit decision after audit; no automation may promote it.

## Classification: separate facts from hypotheses

- **Verified**: link to immutable commit/tree/blob, complete command, environment, raw output/artifact and pass/fail comparison.
- **Inherited failure**: reproduce at pinned Orca source *and* FineCore with matching versions and environment; cite equivalent test name and failure signature. Inherited does not mean passed.
- **Unverified**: missing log, unavailable source, incomplete coverage, no comparable baseline, or unstable/flaky outcome. Mark BLOCKED, never infer success.
- **External evidence**: Pi, Marshall, papers or upstream benchmarks may justify a local experiment, not a FineCore performance claim; record transfer assumptions and limitations.

## Gate A — source, lineage, scope (required before edits)

1. Record `git status --porcelain=v1`, `git branch --show-current`, `git rev-parse HEAD`, `git remote -v`, and `git merge-base` against staging. Do not overwrite dirty files or assume current working tree belongs to staging.
2. Pin upstream Orca commit and verbatim import commit; compare the complete source manifest, count and blob SHA. Baseline at this revision: Orca `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`, initial import `e036dd7a7e2a040b4da1152d99aede0552cb1641`, 1,101 source files. These are recorded reference facts, **not perpetual expected counts**.
3. Identify changed paths against the correct ancestor, and prove that proposed branch descends from staging; default PR base for harness changes is `migration/orca-faithful-import` or a reviewed descendant, never `main`.
4. Inventory every affected source blob and identity change in `docs/identity-divergence.csv`, with truthful classification, justification, and corresponding destination hash. Validate the whole current manifest, including new paths, not only selected rows. Keep historical source artifacts intact.
5. If branch ancestry or manifests conflict, stop modifications and report exact refs, paths, mismatch counts, and reproducer.

## Gate B — canonical identity and compatibility

Canonical **public** identity is `FineCore`: CLI executable/crate `finecore`, UI/help/version, user-facing commands, new documentation, new benchmark-result metadata and future distribution. This does **not** authorize mechanically renaming inherited Rust libraries `orca-harness-*`, `ORCA_*` variables, `.orca` persistence, plugin namespace `io.github.okikorg.orcacode`, protocol headers, MCP identities, or historical fixture/result identifiers.

For each affected surface, maintain an identity contract row: source symbol/path; canonical public name; retained compatibility name; observed behavior; reader/writer precedence; storage/security implications; tests; planned rollback. Alias changes require round-trip and backward-compatibility tests, no silent credential/session migrations or deletion. Historical upstream measurements and release links must be labeled upstream; never present them as measured FineCore performance or available FineCore install methods. FineCore updater/release channels must fail closed until their ownership and artifacts are separately approved. Do not claim npm names are reserved without registry/publisher evidence; a placeholder must be `private: true` or removed.

## Gate C — experimental and regression proof

- Classify each change as **documentation-only**, **identity/compatibility**, **behavioral**, **performance**, or **release/security**. Declare a focused acceptance matrix *before* implementation.
- For behavior/performance changes require trustworthy reproduced upstream tests, directly applicable independent evidence or a local controlled experiment; document baseline, workload, tool versions, sample size, variance/CI where appropriate, pass criteria, safety and negative cases. Re-run FineCore-specific regressions after adaptation. No unmeasured claims.
- Verify `git diff --check`, manifests/hashes, schema/JSON and targeted checks even for docs-only changes. Runtime/config changes also require formatting, build, CLI/workspace tests and relevant Linux/Windows paths, with documented differences against pinned Orca.
- Known historical red gates (Bun REPL, strict Clippy, source-size, binary-size, Windows platform failures) stay **RED** until independently fixed and revalidated. Never weaken budgets, skip tests, or relabel an inherited failure as PASS.
- Distinguish prior stored test logs from current execution, and mock auth from live authenticated behavior. Never expose credentials, run paid-provider calls or destructive state changes without authorization.

## Gate D — review, publication, promotion

1. Commit only scoped, audited files in isolated branch descended from staging. Preserve original authorship, notices and licenses; no force-push. Do not modify unrelated local work.
2. Open **Draft PR** against staging (or appropriate descendant), link upstream ref, divergence rows, test logs, baseline comparisons and explicit RED/UNKNOWN gates. Research-only findings get an issue with falsifiable protocol, not speculative production PR.
3. No checks shown on GitHub is **NOT VERIFIED**, not CI passing. `mergeable: true` is only a Git conflict check, not readiness. Never auto-merge or auto-close.
4. Promotion from staging to bootstrap requires: complete 61-divergence re-review for the pinned snapshot (or updated audited count), identity acceptance matrix, independently refreshed test evidence, resolved/accepted inherited gates with named owner, and explicit human approval. A bootstrap promotion does not authorize touching remote `main`.

## Required completion report

Record: source and destination SHAs; branch and PR base; exact changed paths; blob/manifests deltas; commands run with environment/results; inherited failures and blockers; identity/compatibility implications; upstream/external evidence and limitations; actions explicitly **not** taken (especially `main`, merge, release and force-push). Never say “validated” without specifying which gate and evidence.
