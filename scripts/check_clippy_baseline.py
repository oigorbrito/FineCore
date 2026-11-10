#!/usr/bin/env python3
"""Keep pinned Orca Clippy diagnostics visible and reject new occurrences."""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import Counter
from pathlib import Path


def diagnostic_keys(stream, lint: str) -> tuple[Counter[tuple[str, int, int]], list[str]]:
    """Read Cargo JSON and collect one lint plus any non-baseline errors."""
    found: Counter[tuple[str, int, int]] = Counter()
    errors: list[str] = []
    for number, line in enumerate(stream, 1):
        try:
            message = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"invalid Cargo JSON on line {number}: {error}") from error
        if message.get("reason") != "compiler-message":
            continue
        diagnostic = message.get("message", {})
        code = (diagnostic.get("code") or {}).get("code")
        primary_spans = [span for span in diagnostic.get("spans", []) if span.get("is_primary")]
        if code != lint:
            if diagnostic.get("level") == "error":
                errors.append(diagnostic.get("rendered") or diagnostic.get("message", "unknown error"))
            continue
        if not primary_spans:
            errors.append(diagnostic.get("rendered") or "Clippy diagnostic has no primary span")
        for span in primary_spans:
            path = span["file_name"].replace("\\", "/")
            path = re.sub(r"^.*?(?=crates/)", "", path) if "crates/" in path else path
            found[(path, span["line_start"], span["column_start"])] += 1
    return found, errors


def load_baseline(path: Path) -> Counter[tuple[str, int, int]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    return Counter(
        (item["path"], int(item["line"]), int(item["column"]))
        for item in data["findings"]
    )


def compare(
    current: Counter[tuple[str, int, int]],
    baseline: Counter[tuple[str, int, int]],
) -> tuple[list[tuple[str, int, int]], int]:
    new = list((current - baseline).elements())
    inherited_remaining = sum((current & baseline).values())
    return new, inherited_remaining


def should_fail(
    current: Counter[tuple[str, int, int]],
    baseline: Counter[tuple[str, int, int]],
    unexpected_errors: list[str],
    cargo_exit_code: int,
) -> bool:
    new, inherited = compare(current, baseline)
    return bool(new or unexpected_errors or (cargo_exit_code != 0 and not inherited))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cargo_json", type=Path, help="Cargo --message-format=json output")
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--cargo-exit-code", type=int, required=True)
    args = parser.parse_args()

    try:
        with args.cargo_json.open(encoding="utf-8") as stream:
            current, unexpected_errors = diagnostic_keys(stream, "clippy::result_large_err")
        baseline = load_baseline(args.baseline)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError, KeyError, ValueError) as error:
        print(f"error reading Clippy baseline results: {error}", file=sys.stderr)
        return 2

    new, inherited = compare(current, baseline)
    if new:
        print(f"Found {len(new)} new Clippy diagnostic(s).", file=sys.stderr)
    if unexpected_errors:
        print("Clippy produced errors outside the accepted baseline:", file=sys.stderr)
        for error in unexpected_errors:
            print(error.rstrip(), file=sys.stderr)
    if should_fail(current, baseline, unexpected_errors, args.cargo_exit_code):
        print(f"Clippy failed with exit code {args.cargo_exit_code}.", file=sys.stderr)
        return 1
    baseline_remaining = baseline.copy()
    for path, line, column in sorted(current.elements()):
        key = (path, line, column)
        status = "BASELINE" if baseline_remaining[key] else "NEW"
        baseline_remaining[key] -= 1
        print(f"{status:8} {path}:{line}:{column} clippy::result_large_err")
    if new:
        return 1
    print(f"No new clippy::result_large_err diagnostics; {inherited} inherited remain visible.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
