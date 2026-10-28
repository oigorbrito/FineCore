# FineCore agent operating contract

This file is the root entry point for automated and human-assisted work on the FineCore **harness** staging lineage. For identity and migration work, read and follow [migration/identity skill](.agents/skills/finecore-migration-identity/SKILL.md) before any repository mutation. Instructions here do not imply passing tests or a license to modify protected history.

## Repository histories — fail closed

- Remote `main` is tracker/evidence history; **never** use it as harness integration destination, merge into it, force-push it, rewrite it, or mix unrelated Git histories.
- `bootstrap/finecore-independent` is the greenfield base; `migration/orca-faithful-import` is its Orca harness staging descendant. Develop on isolated descendants of staging; PR Draft base must be staging or an explicitly reviewed descendant, **never main**.
- No merge or auto-merge to any branch, no bootstrap promotion, release, tag, publish, branch protection change, worktree/alias deletion, or destructive rebase without a separate explicit decision. Respect dirty worktrees and existing PR stacks.

## Definition of evidence

Every change must have a source commit and scope, tests or a documented research protocol, and a clear distinction between observed behavior, upstream data and inference. An external Pi/Marshall/Orca benchmark is not a FineCore benchmark. No passing status from historical logs. Inherited failing gates remain RED until fixed, not waived by a comparison.

## Canonical product identity

Public presentation and new artifacts use **FineCore**. Keep inherited `orca-harness-*` public library identifiers, `ORCA_*` environment variables, persisted config paths, plugin namespaces and protocol contracts where compatibility requires them. Do not globally replace `orca` strings, rewrite historical Orca results or imply upstream downloads install FineCore. For any new alias, specify precedence, old-data read compatibility, test and rollback; never silently touch credentials.

## Required work protocol

1. Read the skill and referenced migration manifests; inspect Git branch, SHA, ancestry and dirty state before edits.
2. Search for equivalent upstream or existing work; define narrow change scope and executable acceptance criteria before code. If evidence is insufficient, create a research issue instead of untested functionality.
3. Keep source/import and identity-divergence inventories mechanically reconcilable. Preserve license/NOTICE and provenance.
4. Validate focused regression tests plus impacted broader gates. Report exact commands, OS/toolchain, results, skipped tests and inherited failures. Never disable or relax a failing gate just to obtain PASS.
5. Open Draft PR on correct staging base with immutable references, scope, tests and remaining blockers. No automatic merge.
6. End with concise evidence table and explicit statement of what branches/actions remained untouched.

If anything in this file conflicts with actual Git ancestry or test evidence: **STOP, report the conflict, do not guess and do not mutate protected refs.**

## Source of truth

- `docs/upstream-import-manifest.csv` — verbatim source comparison.
- `docs/identity-divergence.csv` — classified identity differences.
- `docs/FINECORE_IDENTITY_INVENTORY.md` — compatibility surfaces and decisions.
- `docs/MIGRATION_VALIDATION.md` — recorded, not necessarily current, validation.
- `docs/PI_MODEL_AUTH_PARITY_ROADMAP.md` — deferred feature roadmap and identity 0A proposed separately in PR #132.
