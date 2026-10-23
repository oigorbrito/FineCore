# FINECORE-M01 — migration ledger

Status: **IN PROGRESS — NOT VALIDATED**

## Immutable starting points

- Source: `oigorbrito/core` / `feat/core-runtime-foundation-001` @ `8ca24d268ed78d71e326b1f6ac53255d1a723928`.
- Destination: `oigorbrito/finecore` / `main` @ `d932e68ecdf49cc1d51b344aa629659cd46d0997`.
- Work branch: `migration/finecore-m01`. No automatic merge.

## Initial inventory (GitHub recursive trees)

- Source tree: 166 entries (directories and files), not truncated.
- Destination tree: 31 entries, not truncated.
- Destination contains `README.md` and `evidence-repo-template/`; those paths are not safe to overwrite without blob-by-blob reconciliation.
- Source includes runtime, benchmarks, experiments, workflows and an evidence template. None may be dropped without an explicit disposition.
- A successful contract workflow is **not** evidence of superior real-task efficacy. Local paired Pi/Orca efficacy remained blocked by host load.

## Gates

| Gate | State |
| --- | --- |
| Pin source and destination | PASS |
| Identify destination-owned paths | PASS (initial tree inventory) |
| Full per-blob collision reconciliation | PENDING |
| Transfer source implementation | PENDING |
| Canonical identity rename | PENDING |
| Tests and CI on migrated tree | PENDING |
| Review and manual merge | PENDING |

The migration is incomplete until every pending gate has traceable evidence. No claims of full functional parity or benchmark improvement are authorized.
