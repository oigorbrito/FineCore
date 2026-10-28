---
name: orca-harness-migration
description: Guide proposed FineCore migration work from pinned Orca Harness upstream. Use for migration planning, full-tree import reconciliation, identity-only changes, audits, and validation.
---

# FineCore / Orca Harness migration executor

Use `docs/MIGRATION_ROADMAP.md` as a proposal subordinate to accepted staging instructions until formally reconciled. This skill keeps work bounded and auditable; it does not authorize reuse, destructive operations, publication, or branch promotion.

## Scope and authority

- FineCore is the canonical public product identity. Preserve upstream code and compatibility contracts as closely as possible; change identity only through reviewed, narrow decisions.
- The canonical Orca import is documented on `migration/orca-faithful-import`, descended from the greenfield `bootstrap/finecore-independent`. This skill and roadmap are proposals under review; before adoption, reconcile them against applicable instructions and evidence at the actual staging revision, recording paths, revisions, authority, and unresolved conflicts. Until accepted, follow applicable instructions at the revision used. Do not repeat the import into bootstrap; promotion is separate and requires explicit approval.
- Split remaining migration work into focused PRs by phase or independently verifiable deliverable. Keep commits coherent and defensible for each scope; do not impose a fixed count or fragment commits without review/validation value. Make dependencies and PR bases explicit.

## Fixed source reference

- Historical source reference: `https://github.com/okikorg/orca-harness`, commit `2a9fd399cfbbaad773e3cbed92bb6e24d7561f01`, tree `9c35d0dd0d7e0f226063f16dd80161e231628e2b`. Reconcile with current staging provenance before follow-up work.
- Read-only comparison clone: `../orca-harness-reference`, local branch `reference/orca-harness-2a9fd39`, checked out at that pin. Do not merge its Git history into FineCore.
- Historical tree inventory: 1,284 recursive entries including directories and 1,101 file paths; verify manifest counts before treating these as current.
- Staging destination: FineCore branch `migration/orca-faithful-import`; verify current head and ancestry before work. Bootstrap is a greenfield base, not the import destination.
- Goal: complete upstream tree plus only approved canonical identity changes. No feature selection, optimization, or behavior/API changes in this migration.
- Preserve required license and attribution notices. A public repository is not permission to remove notices or misrepresent provenance. If obligations are unclear, block only the affected import, redistribution, or transformation; record the question and continue independent work. Request a decision only when needed for that action.

The source reference clone and commit/tree/counts above are historical records. Verify their availability and applicability to the current work before reuse or import, recording the evidence. Label details not verified for the current work accordingly.

## Before each work session

1. Read the roadmap and any repository-specific agent instructions.
2. Run `git status --short --branch`; inspect current branch, commit, remotes, and refs. Never discard unrelated changes.
3. State the current phase, phase gate, intended files/actions, and validation before editing.
4. Do not switch branches, rewrite history, run destructive Git commands, commit, push, publish, promote staging to bootstrap, or change `main` unless the user explicitly authorizes that exact action.

## Phase discipline

- Respect dependencies among applicable phases: preflight -> baseline evidence -> import reconciliation -> minimal identity layer -> validation -> audit/handoff. A blocked dependency does not stop independent work. The full import is already recorded in staging; verify existing manifests and update them only for demonstrated gaps or new changes rather than repeating the import or creating parallel inventories.
- Do not perform import-dependent mutations before source pin/tree and destination ref are verified, required notices are accounted for, and relevant identity changes have scoped manifest entries and review. Existing divergence rows do not automatically authorize new transformations.
- Run and record pristine upstream tests before identity edits where applicable evidence is not already available. Reconcile existing staging logs first: verify source/destination SHAs, toolchain, platform, and scope; repeat only uncovered or stale checks. Record unavailable checks as `BLOCKED` or `NOT RUN`; they block only claims or actions that depend on them, not independent audit/documentation work. Benchmarks are a separate optional track unless performance claims are in scope. Do not silently repair upstream failures.
- Account for every tracked source path, including tests, benchmarks, scripts, workflows, docs, generated files and assets. Any omission needs a concrete reason and approval.
- Keep the identity transformation narrow. No global replacement without reviewing its effects. Do not change behavior, public APIs, dependencies, tests, benchmark logic, or workflows except for an explicitly approved identity consequence.
- Treat staging import/divergence manifests as source of truth; do not create parallel inventories unless a demonstrated gap requires it. After each phase, compare paths/blobs and report every divergence with category, reason, and evidence. Keep unexplained differences as blockers to dependent mutations while continuing independent audit/documentation work.
- Run the upstream-prescribed tests, clippy and formatting commands on the migrated result; compare with pristine baseline. Never report unrun checks as passing or hide failures.
- Compare performance only using comparable workloads, tools, environment and repeated measurements. This phase validates preservation; it is not permission to optimize.

## Blockers and continuation

Record a blocker and its evidence, owner or resolution condition when source/destination revisions differ from the pin, an omission or behavior change appears necessary, provenance/notices are uncertain, checks require credentials or external side effects, a benchmark cannot be made comparable, or a regression exceeds an agreed threshold. Do not perform the action that depends on the unresolved item. Continue independent checks and deliverables; ask the user only when their decision is required to proceed with a specific dependent action.

## Status report format

At the start/end of a task, report: current phase; source/destination revision; files changed; commands and exact outcomes; divergence/exception count; blockers and approvals needed; next gate. Separate verified facts from assumptions. Required checks must pass for `MIGRATION_COMPLETE`. Do not use `MIGRATION_COMPLETE_WITH_INHERITED_RED_GATES` to waive a gate: under the accepted identity policy, historical inherited reds remain red until independently fixed and revalidated. If another accepted staging policy defines a distinct status, follow its exact conditions without describing red checks as passing. Missing required evidence, migration-introduced or unresolved regressions, and unverified or red required gates remain pending. List external/platform gaps with status and resolution conditions. Benchmark completion is required only for benchmark/performance claims. Promotion, merge, and release approvals remain separate from migration completion.
