#!/usr/bin/env python3
"""Report Rust source size and reject new or growing over-limit files.

The audit intentionally measures physical lines for the limit, matching the
existing ``ci/check-source-size.sh`` check: a file may have at most 599 lines
(it must stay below 600).  Non-blank lines are included as a second useful
measure when reviewing a large file.

The pinned Orca import contains files that already exceed the limit. A checked-
in baseline records those exact line counts. Existing debt stays visible, while
new over-limit files and growth beyond a recorded count fail the gate.
``--top`` keeps the report to the largest files, always including failures.

Usage:
    python3 scripts/audit_crates.py
    python3 scripts/audit_crates.py --top 25
    python3 scripts/audit_crates.py --root /path/to/orca-harness --limit 599
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path


# Same number as ci/check-source-size.sh: more than this many lines fails.
DEFAULT_LIMIT = 599
DEFAULT_BASELINE = Path(__file__).resolve().parents[1] / "ci/baselines/orca-d89bf274.json"


@dataclass(frozen=True)
class FileStats:
    path: str
    total_lines: int
    non_blank_lines: int


def count_lines(path: Path) -> tuple[int, int]:
    """Return newline counts (matching ``wc -l``) and non-blank line counts."""
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    return text.count("\n"), sum(bool(line.strip()) for line in lines)


def audit_crates(root: Path, limit: int) -> list[FileStats]:
    """Collect source statistics below ``root/crates``."""
    crates = root / "crates"
    if not crates.is_dir():
        raise FileNotFoundError(f"crates directory not found: {crates}")

    stats = []
    for suffix in ("*.rs", "*.inc", "*.part"):
        for path in sorted(crates.rglob(suffix)):
            if not path.is_file():
                continue
            total, non_blank = count_lines(path)
            stats.append(FileStats(path.relative_to(root).as_posix(), total, non_blank))

    return stats


def load_baseline(path: Path) -> dict[str, int]:
    """Load the exact imported-source line-count ceilings."""
    data = json.loads(path.read_text(encoding="utf-8"))
    return {str(name): int(lines) for name, lines in data["source_line_counts"].items()}


def classify_size(path: str, lines: int, limit: int, baseline: dict[str, int]) -> str:
    """Return ``ok``, ``baseline``, or ``regression`` for a source file."""
    if lines <= limit:
        return "ok"
    if path in baseline and lines <= baseline[path]:
        return "baseline"
    return "regression"


def print_report(
    root: Path,
    stats: list[FileStats],
    limit: int,
    baseline: dict[str, int],
    top: int | None = None,
) -> int:
    """Print the report and return the number of new or worsened violations.

    ``top`` limits the table to the ``top`` largest files; flagged files are
    never dropped, since they are the reason the report exists.
    """
    ordered = sorted(stats, key=lambda entry: (-entry.total_lines, entry.path))
    states = {
        item.path: classify_size(item.path, item.total_lines, limit, baseline)
        for item in ordered
    }
    flagged = sum(state == "regression" for state in states.values())
    inherited = sum(state == "baseline" for state in states.values())
    shown = ordered if top is None else ordered[: max(top, flagged)]

    print(f"Crate source files: {len(stats)}")
    print(f"Limit:            > {limit} physical lines")
    if len(shown) < len(ordered):
        print(f"Showing:          {len(shown)} largest of {len(ordered)}")
    print()
    print(f"{'Status':10} {'Lines':>7} {'Non-blank':>10}  File")
    print("-" * 72)

    for item in shown:
        status = states[item.path].upper()
        print(
            f"{status:10} {item.total_lines:7} {item.non_blank_lines:10}  "
            f"{item.path}"
        )

    print()
    if flagged:
        print(f"Found {flagged} new or worsened violation(s) over {limit} lines.", file=sys.stderr)
    else:
        print(f"No new or worsened violations over {limit} lines.")
    if inherited:
        print(f"Existing imported baseline debt: {inherited} file(s) over {limit} lines.")
    return flagged


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
        help="repository root (default: inferred from this script)",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=DEFAULT_LIMIT,
        help=f"flag files with more than this many lines (default: {DEFAULT_LIMIT})",
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        default=DEFAULT_BASELINE,
        help="JSON file containing exact inherited source line-count ceilings",
    )
    parser.add_argument(
        "--top",
        type=int,
        default=None,
        help="list only the N largest files (flagged files are always listed)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.limit < 0:
        print("error: --limit must be non-negative", file=sys.stderr)
        return 2
    if args.top is not None and args.top < 0:
        print("error: --top must be non-negative", file=sys.stderr)
        return 2
    try:
        stats = audit_crates(args.root, args.limit)
        baseline = load_baseline(args.baseline)
    except (FileNotFoundError, UnicodeDecodeError, KeyError, ValueError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 1 if print_report(args.root, stats, args.limit, baseline, args.top) else 0


if __name__ == "__main__":
    raise SystemExit(main())
