# FineCore — engineering instructions

Scope: repository root. This file defines migration safeguards, not agent architecture.

- Canonical project identity: **FineCore**; repository: `oigorbrito/finecore`. Preserve technical uses of “core” that denote concepts, APIs, or upstream identifiers.
- Source of migration evidence: `oigorbrito/core`, branch `feat/core-runtime-foundation-001`, pinned commit `8ca24d268ed78d71e326b1f6ac53255d1a723928`. Do not silently update the source pin.
- Inventory source and destination trees before copying; classify every collision as identical, rename, destination-owned, or unresolved.
- Preserve behavior, test fixtures, original benchmark artifacts, and upstream attribution. Do not infer Pi/Orca superiority from contract tests or blocked local trials.
- Keep changes on the migration branch. No force-push, deletion, automatic merge, or changes to protected `main`.
- Verify provenance, renamed references, workflow syntax, executable tests and baseline parity in grouped gates; label missing evidence `UNKNOWN`, never `PASS`.
- Keep instructions short and executable. Prefer one migration report over proliferating markdown files.
- For reconciliation procedure, consult `.github/skills/reconciliation/SKILL.md`. Do not load unrelated skills.
