# FineCore agent operating contract

This file is the root entry point for automated and human-assisted work on the FineCore **harness** staging lineage. For identity and migration work, read and follow [migration/identity skill](.agents/skills/finecore-migration-identity/SKILL.md) before any repository mutation. Instructions here do not imply passing tests or a license to modify protected history.

## Repository histories — fail closed

- Remote `main` is tracker/evidence history; **never** use it as harness integration destination, merge into it, force-push it, rewrite it, or mix unrelated Git histories.
- `bootstrap/finecore-independent` is the greenfield base; `migration/orca-faithful-import` is its Orca harness staging descendant. Develop on isolated descendants of staging; PR Draft base must be staging or an explicitly reviewed descendant, **never main**.
- No merge or auto-merge to any branch, no bootstrap promotion, release, tag, publish, branch protection change, worktree/alias deletion, or destructive rebase without a separate explicit decision. Respect dirty worktrees and existing PR stacks.

## Rule provenance and review status

The [skill evidence and authority map](.agents/skills/finecore-migration-identity/SKILL.md#evidence-and-authority-map) states for each gate whether its basis is a technical contract, the repository's recorded evidence, or FineCore-specific governance. Cite the exact revision and command/output for PASS; do not infer current validation from archived logs. GitHub mergeability alone does not establish approval, test success, or compliance. FineCore's no-automatic-merge and history separation rules are deliberate governance restrictions, **not claims that SLSA, NIST or Git requires this exact workflow**. This policy references established practices but makes **no certification claim**.

## Definition of evidence

Every change must have a source commit and scope, tests or a documented research protocol, and a clear distinction between observed behavior, upstream data and inference. An external Pi/Marshall/Orca benchmark is not a FineCore benchmark. No passing status from historical logs. Inherited failing gates remain RED until fixed, not waived by a comparison.

## Canonical product identity

Public presentation and new artifacts use **FineCore**. Keep inherited `orca-harness-*` public library identifiers, `ORCA_*` environment variables, persisted config paths, plugin namespaces and protocol contracts where compatibility requires them. Do not globally replace `orca` strings, rewrite historical Orca results or imply upstream downloads install FineCore. For any new alias, specify precedence, old-data read compatibility, test and rollback; never silently touch credentials.

## Branch, PR and documentation hygiene

Prefer one scoped PR containing cohesive commits instead of a new numbered branch for each step. Review the commit history as well as the net diff: avoid checkpoint commits for each AI iteration; commit coherent validated slices. At 5 commits in a scoped PR, review churn and justify further commits (a review checkpoint, not a hard limit). Never auto-squash, amend/rebase published history or force-push; propose consolidation for explicit human approval. Use stacked PRs only for real dependencies, document the parent/base in the PR and review stacks after three open layers. The limit is a **review checkpoint**, not a hard empirical threshold. Reuse existing issues, roadmaps and evidence ledgers; do not proliferate status markdown. Store raw logs as linked CI/evidence artifacts rather than redundant repository copies. Never auto-restack/rebase, force-push, retarget, delete or merge; the harness PR trunk is staging, not remote `main`. See the skill's [change hygiene section](.agents/skills/finecore-migration-identity/SKILL.md#change-hygiene--avoid-stacked-pr-and-documentation-sprawl) for justification and exceptions.

PR history circuit breaker: inspect the actual PR merge-base and unique commit count before routine commit/push. **At 20 commits relative to the intended PR base, stop routine commit/push loops and require a human review**, except a documented urgent safety fix. If no common ancestor exists, stop and report topology error. This is project risk control, not a scientifically established number. Existing branches must not be rewritten or deleted automatically. The skill alone cannot enforce this server-side; a separately approved required CI/status check would be needed.

## Required work protocol

1. Read the skill and referenced migration manifests; inspect Git branch, SHA, ancestry and dirty state before edits.
2. Search for equivalent upstream or existing work; define narrow change scope and executable acceptance criteria before code. If evidence is insufficient, create a research issue instead of untested functionality.
3. Keep source/import and identity-divergence inventories mechanically reconcilable. Preserve license/NOTICE and provenance.
4. Validate focused regression tests plus impacted broader gates. Report exact commands, OS/toolchain, results, skipped tests and inherited failures. Never disable or relax a failing gate just to obtain PASS.
5. Open Draft PR on correct staging base with immutable references, scope, tests and remaining blockers. No automatic merge.
6. End with concise evidence table and explicit statement of what branches/actions remained untouched.

If anything in this file conflicts with actual Git ancestry or test evidence: **STOP, report the conflict, do not guess and do not mutate protected refs.**


## Document access and historical reconciliation

The executor must use files **in its checked-out branch**, not documents from chat history or another un-fetched branch. Resolve the repository root first; if a required file is absent, report its exact path/ref and do not silently substitute. Read-only GitHub access to other branches is permitted for evidence gathering; copying or integrating their history is not.

Canonical operational records shipped in this staging lineage: `docs/FINECORE_IDENTITY_INVENTORY.md`, `docs/upstream-import-manifest.csv`, `docs/identity-divergence.csv`, `docs/MIGRATION_VALIDATION.md`, and the separate **Pi feature** roadmap `docs/PI_MODEL_AUTH_PARITY_ROADMAP.md`. The root `AGENTS.md` and `.agents/skills/finecore-migration-identity/SKILL.md` are available in staging following the integration of PR #133. Verify their presence at the exact checked-out revision; this is not evidence that a particular executor has loaded them.

Historical records are **not** current execution authority: `migration/finecore-m01@a0b9d775` holds `AGENTS.md`, `.github/skills/reconciliation/SKILL.md`, `docs/migration/FINECORE-M01.md` for an older migration from `oigorbrito/core@8ca24d26`; `migration/orca-tree-import@41be0191` holds `docs/migration/ORCA-IMPORT-04.md` proposing an Orca subtree under `vendor/orca-harness/`; `feature/pi-auth-parity-02@5b652a84` holds `docs/engineering/ORCA_MIGRATION_INTEGRATION.md` describing PR #111 targeting main. These are different histories/strategies. Inspect them at pinned refs when evaluating provenance; never execute their promotion instructions against current staging without an explicit decision.

PR #134 was merged into staging and added `docs/MIGRATION_ROADMAP.md` plus `.agents/skills/orca-harness-migration/SKILL.md`. The roadmap is the operational phase and gate plan for the Orca migration. This root contract and `.agents/skills/finecore-migration-identity/SKILL.md` govern identity, evidence, branch safety, and policy; the roadmap does not authorize protected-ref changes, merge, or promotion. At every checkout, verify the files and applicable revisions rather than relying on chat context.

For a missing document or conflicting instruction: continue **independent read-only or non-conflicting tasks**, record the missing source as UNKNOWN/BLOCKED for the dependent gate, and halt only mutations that rely on that disputed decision. Never broadly halt safe inventory work or silently assume precedence.

## Source of truth

- `docs/upstream-import-manifest.csv` — verbatim source comparison.
- `docs/identity-divergence.csv` — classified identity differences.
- `docs/FINECORE_IDENTITY_INVENTORY.md` — compatibility surfaces and decisions.
- `docs/MIGRATION_VALIDATION.md` — recorded, not necessarily current, validation.
- `docs/PI_MODEL_AUTH_PARITY_ROADMAP.md` — separate Pi feature roadmap; proposed changes in PR #132 must be checked at the current ref.
- `docs/MIGRATION_ROADMAP.md` — operational Orca migration phase/gate plan, integrated by PR #134; subordinate to this contract and the identity skill.
