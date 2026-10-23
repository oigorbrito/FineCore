---
name: reconciliation
description: Reconcile the pinned Core source with FineCore during the identity migration, preserving provenance, behavior and destination-owned files.
---

# Reconcile FineCore migration

Use this skill for source-to-destination inventory, identity migration and verification; not for ordinary programming.

1. Freeze both commit SHAs and enumerate tracked files and their blob SHAs recursively. Record source-only, destination-only, identical and conflicting paths.
2. Treat destination-owned paths as protected. Never overwrite a conflicting blob without inspecting both versions and recording the disposition.
3. Copy source files faithfully first. Apply canonical identity changes separately and distinguish project branding from generic “core” terminology, external upstream names and public API contracts.
4. Keep reproducible benchmarks and their provenance. Classify offline contract tests, synthetic trials and actual agent task efficacy separately; a blocked trial yields no comparative result.
5. Validate in grouped gates: file inventory and provenance; build/test contracts; reference and naming audit; CI status. If a gate cannot run, report BLOCKED or UNKNOWN with its cause.
6. Produce a single concise reconciliation report with source/destination SHAs, counts, collisions, actual test commands/results, exceptions and remaining work.
7. Do not merge, delete branches, rewrite history or change repository security settings. Manual review decides promotion to main.
