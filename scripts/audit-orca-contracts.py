#!/usr/bin/env python3
"""Inventory identity-sensitive literals in a checked-out Orca subtree.

Read-only; stdlib only. Emits deterministic JSON. Does not execute Rust code.
"""
import argparse
import json
import pathlib
import re
import subprocess
import sys

PATTERNS = {
    "environment": re.compile(r'\b(?:ORCA_[A-Z0-9_]+|XDG_CONFIG_HOME|APPDATA|HOME)\b'),
    "persistence": re.compile(r'(?<![A-Za-z0-9_])(?:config\.json|memory\.sqlite3|recovery\.json|sessions|\.orca)(?![A-Za-z0-9_])'),
    "protocol": re.compile(r'(?<![A-Za-z0-9_])(?:NDJSON|RunEvent|HarnessEvent|serde_json::to_string|--json|--resume|--continue)(?![A-Za-z0-9_])'),
    "identity": re.compile(r'\b(?:orcacode|orca_harness_[a-z_]+|orca-harness-[a-z-]+)\b'),
}
def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default="vendor/orca-harness")
    ap.add_argument("--expected-tree", default="9c35d0dd0d7e0f226063f16dd80161e231628e2b")
    ap.add_argument("--output", default="-")
    args = ap.parse_args()
    root = pathlib.Path(args.root).resolve()
    if not root.is_dir():
        ap.error(f"missing subtree: {root}")
    try:
        relative = root.relative_to(pathlib.Path(git("rev-parse", "--show-toplevel")).resolve()).as_posix()
        tree = git("rev-parse", f"HEAD:{relative}")
    except (ValueError, subprocess.CalledProcessError) as exc:
        ap.error(f"cannot establish Git tree: {exc}")
    if tree != args.expected_tree:
        ap.error(f"tree mismatch: {tree} != {args.expected_tree}")
    # Reject worktree edits and untracked files: HEAD tree equality alone is insufficient.
    changed = subprocess.check_output(
        ["git", "status", "--porcelain=v1", "--untracked-files=all", "--", relative],
        text=True,
    )
    if changed.strip():
        ap.error("donor worktree differs from pinned Git tree")
    tracked = subprocess.check_output(
        ["git", "ls-tree", "-r", "--name-only", "HEAD", "--", relative],
        text=True,
    ).splitlines()
    expected_files = sorted(p[len(relative) + 1:] for p in tracked
                            if p.startswith(relative + "/") and p.endswith(".rs"))
    files = sorted(root.rglob("*.rs"))
    actual_files = [p.relative_to(root).as_posix() for p in files]
    if actual_files != expected_files:
        ap.error(f"Rust file set mismatch: expected {len(expected_files)}, found {len(actual_files)}")
    rows = []
    for file in files:
        for line_no, line in enumerate(file.read_text(encoding="utf-8").splitlines(), 1):
            for category, pattern in PATTERNS.items():
                for m in pattern.finditer(line):
                    rows.append({"path": file.relative_to(root).as_posix(),
                                 "line": line_no, "category": category,
                                 "symbol": m.group(0)})
    result = {"schema": 1, "tree": tree, "files_scanned": len(files),
              "matches": len(rows), "occurrences": rows}
    data = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    if args.output == "-":
        sys.stdout.write(data)
    else:
        pathlib.Path(args.output).write_text(data, encoding="utf-8")
    print(f"ORCA-06B: scanned={len(files)} matches={len(rows)} tree={tree}", file=sys.stderr)

if __name__ == "__main__":
    main()
