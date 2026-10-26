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
MIGRATION_MANIFEST = "docs/identity-divergence.csv"
IMPORT_MANIFEST = "docs/upstream-import-manifest.csv"
IDENTITY_MANIFEST_BLOB = "b79d938a35112d4535fa2e2d50fdc9d0be72b848"


def git(*args: str) -> str:
    p = subprocess.run(["git", *args], cwd=ROOT, text=True,
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if p.returncode:
        raise RuntimeError(f"git {' '.join(args)} failed: {p.stderr.strip()}")
    return p.stdout.strip()


def blob(ref: str, path: str) -> str:
    return git("rev-parse", f"{ref}:{path}")


def read_csv(path: str) -> list[dict[str, str]]:
    with (ROOT / path).open(encoding="utf-8", newline="") as f:
        return list(csv.DictReader(f))


def main() -> None:
    head = git("rev-parse", "HEAD")
    git("merge-base", "--is-ancestor", MAIN, head)
    assert blob("HEAD", MIGRATION_MANIFEST) == IDENTITY_MANIFEST_BLOB, (
        "Pinned divergence manifest was modified; review its new content first"
    )

    imported = read_csv(IMPORT_MANIFEST)
    assert len(imported) == 1101, f"expected 1101 imported paths, got {len(imported)}"
    original_paths: set[str] = set()
    for row in imported:
        path = row["destination_path"]
        assert path not in original_paths, f"duplicate import path: {path}"
        original_paths.add(path)
        assert blob(PINNED_DONOR_STAGE, path) == row["source_blob"], path
        assert row["source_blob"] == row["destination_blob"], path

    divergent = read_csv(MIGRATION_MANIFEST)
    assert len(divergent) == 1116, f"expected 1116 divergence rows, got {len(divergent)}"
    tally: Counter[str] = Counter()
    seen: set[str] = set()
    for row in divergent:
        path = row["destination_path"]
        assert path not in seen, f"duplicate divergence path: {path}"
        seen.add(path)
        status = row["status"]
        tally[status] += 1
        assert blob("HEAD", path) == row["destination_worktree_blob"], path
        if status == "identical":
            assert row["source_blob"] == row["destination_worktree_blob"], path
        elif status == "modified":
            assert path in original_paths, path
            assert row["source_blob"] != row["destination_worktree_blob"], path
        elif status == "added":
            assert not row["source_blob"] and path not in original_paths, path
        else:
            raise AssertionError(f"Unclassified divergence: {path}: {status}")
    assert tally == Counter({"identical": 1040, "modified": 61, "added": 15}), tally
    assert original_paths <= seen, f"missing donor paths: {len(original_paths - seen)}"

    # Main-only governance and independent tracker must survive verbatim.
    main_paths = git("ls-tree", "-r", "--name-only", MAIN).splitlines()
    main_only = [p for p in main_paths if p not in original_paths]
    assert len(main_only) == 25, f"expected 25 main-only paths, got {len(main_only)}"
    for path in main_only:
        assert blob("HEAD", path) == blob(MAIN, path), (
            f"main governance path unexpectedly changed: {path}"
        )

    # Original published migration snapshot contributes its own CSV file.
    assert MIGRATION_MANIFEST not in seen
    print(
        "INTEGRATION_SOURCE_AUDIT_PASS",
        f"head={head}",
        f"donor=1101",
        f"identical={tally['identical']}",
        f"modified={tally['modified']}",
        f"added_in_manifest={tally['added']}",
        "plus_manifest_itself=1",
        f"main_only_preserved={len(main_only)}",
        sep="\n",
    )


if __name__ == "__main__":
    main()
