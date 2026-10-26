#!/usr/bin/env python3
"""Offline, read-only Git-object audit of the FineCore integration review tree."""
from __future__ import annotations

import csv
import subprocess
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAIN = "95e666924e672dba4749ab02b26d86782b0bf78a"
PINNED_DONOR_STAGE = "25bb2a705be2f6903c52b3a2507159b9d98b4a89"
MIGRATION_HEAD = "2d95a81054734730715369d54f64b28f419734fc"
MIGRATION_MANIFEST = "docs/identity-divergence.csv"
IMPORT_MANIFEST = "docs/upstream-import-manifest.csv"
OVERLAY_MANIFEST = "docs/integration-divergence.csv"
IDENTITY_MANIFEST_BLOB = "b79d938a35112d4535fa2e2d50fdc9d0be72b848"
OVERLAY_MANIFEST_BLOB = "b57edbea38a96106ed3a81e893321c8e40c73e8d"
INTEGRATION_CONTROLS = {
    ".github/workflows/finecore-integration-review.yml",
    "docs/engineering/ORCA_MIGRATION_INTEGRATION.md",
    "docs/integration-divergence.csv",
    "scripts/verify_finecore_integration.py",
}


def git(*args: str) -> str:
    p = subprocess.run(["git", *args], cwd=ROOT, text=True,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if p.returncode:
        raise RuntimeError(f"git {' '.join(args)} failed: {p.stderr.strip()}")
    return p.stdout.strip()


def tree_entries(ref: str) -> dict[str, tuple[str, str]]:
    entries: dict[str, tuple[str, str]] = {}
    for line in git("ls-tree", "-r", "--full-tree", ref).splitlines():
        metadata, separator, path = line.partition("\t")
        assert separator and path, (ref, line)
        mode, kind, oid = metadata.split()
        assert kind == "blob", (ref, path, kind)
        assert path not in entries, f"duplicate tree path in {ref}: {path}"
        entries[path] = (mode, oid)
    return entries


def read_csv(path: str) -> list[dict[str, str]]:
    with (ROOT / path).open(encoding="utf-8", newline="") as f:
        return list(csv.DictReader(f))


def main() -> None:
    head = git("rev-parse", "HEAD")
    git("merge-base", "--is-ancestor", MAIN, head)

    main_tree = tree_entries(MAIN)
    donor_stage_tree = tree_entries(PINNED_DONOR_STAGE)
    migration_tree = tree_entries(MIGRATION_HEAD)
    combined_tree = tree_entries("HEAD")

    assert migration_tree.get(MIGRATION_MANIFEST) == (
        "100644", IDENTITY_MANIFEST_BLOB
    ), "Accepted migration manifest at its pinned commit changed"
    assert combined_tree.get(OVERLAY_MANIFEST) == (
        "100644", OVERLAY_MANIFEST_BLOB
    ), "Integration overlay manifest changed; review and repin it"

    imported = read_csv(IMPORT_MANIFEST)
    assert len(imported) == 1101, f"expected 1101 imported paths, got {len(imported)}"
    original_paths: set[str] = set()
    for row in imported:
        path = row["destination_path"]
        assert path not in original_paths, f"duplicate import path: {path}"
        original_paths.add(path)
        staged = donor_stage_tree.get(path)
        assert staged is not None and staged[1] == row["source_blob"], path
        assert row["source_blob"] == row["destination_blob"], path

    divergent = read_csv(MIGRATION_MANIFEST)
    assert len(divergent) == 1116, f"expected 1116 migration rows, got {len(divergent)}"
    tally: Counter[str] = Counter()
    seen: set[str] = set()
    for row in divergent:
        path = row["destination_path"]
        assert path not in seen, f"duplicate migration path: {path}"
        seen.add(path)
        status = row["status"]
        tally[status] += 1
        migration_entry = migration_tree.get(path)
        assert migration_entry is not None and migration_entry[1] == row["destination_worktree_blob"], path
        if status == "identical":
            assert row["source_blob"] == row["destination_worktree_blob"], path
        elif status == "modified":
            assert path in original_paths, path
            assert row["source_blob"] != row["destination_worktree_blob"], path
        elif status == "added":
            assert not row["source_blob"] and path not in original_paths, path
        else:
            raise AssertionError(f"Unclassified migration divergence: {path}: {status}")
    assert tally == Counter({"identical": 1040, "modified": 61, "added": 15}), tally
    assert original_paths <= seen, f"missing donor paths: {len(original_paths - seen)}"

    overlay = read_csv(OVERLAY_MANIFEST)
    assert len(overlay) == 5, f"expected five integration corrections, got {len(overlay)}"
    overlay_paths: set[str] = set()
    for row in overlay:
        path = row["path"]
        assert path not in overlay_paths, f"duplicate integration overlay path: {path}"
        overlay_paths.add(path)
        assert path in seen, f"overlay path is outside the migration inventory: {path}"
        expected_before = (row["migration_mode"], row["migration_blob"])
        expected_after = (row["integration_mode"], row["integration_blob"])
        assert migration_tree.get(path) == expected_before, path
        assert combined_tree.get(path) == expected_after, path
        assert expected_before != expected_after, f"overlay row is not a change: {path}"

    migration_paths = set(migration_tree)
    combined_paths = set(combined_tree)
    assert len(migration_paths) == 1117, f"expected 1117 migration paths, got {len(migration_paths)}"
    assert migration_paths <= combined_paths, f"integration dropped {len(migration_paths - combined_paths)} migration paths"
    for path in migration_paths - overlay_paths:
        assert combined_tree[path] == migration_tree[path], f"unrecorded integration change: {path}"

    # Preserve main-only governance and independent tracker files verbatim.
    main_paths = set(main_tree)
    main_only = main_paths - original_paths
    assert len(main_only) == 25, f"expected 25 main-only paths, got {len(main_only)}"
    for path in main_only:
        assert combined_tree.get(path) == main_tree[path], (
            f"main governance path unexpectedly changed: {path}"
        )
    collisions = main_paths & original_paths
    assert collisions == {"README.md"}, f"unexpected source/main path collisions: {collisions}"

    added_integration_paths = combined_paths - migration_paths
    assert added_integration_paths == main_only | INTEGRATION_CONTROLS, (
        f"unexpected integration additions: {sorted(added_integration_paths ^ (main_only | INTEGRATION_CONTROLS))}"
    )
    assert len(combined_paths) == 1146, f"expected 1146 combined paths, got {len(combined_paths)}"
    assert MIGRATION_MANIFEST not in seen

    print(
        "INTEGRATION_SOURCE_AUDIT_PASS",
        f"head={head}",
        f"main_parent={MAIN}",
        "donor=1101",
        f"identical={tally['identical']}",
        f"modified={tally['modified']}",
        f"added_in_migration_manifest={tally['added']}",
        f"integration_overlay={len(overlay_paths)}",
        f"main_only_preserved={len(main_only)}",
        f"combined_paths={len(combined_paths)}",
        sep="\n",
    )


if __name__ == "__main__":
    main()
