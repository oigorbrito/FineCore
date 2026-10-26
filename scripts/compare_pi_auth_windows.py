#!/usr/bin/env python3
"""Run and compare the full FineCore CLI suite on two checked-out revisions."""

from __future__ import annotations

import argparse
import json
import platform
import re
import subprocess
from pathlib import Path

SUMMARY = re.compile(
    r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored"
)


def command_version(command: list[str]) -> str:
    result = subprocess.run(command, text=True, capture_output=True, check=True)
    return result.stdout.strip()


def commit_for(path: Path) -> str:
    return command_version(["git", "-C", str(path.resolve()), "rev-parse", "HEAD"])


def failure_names(log: str) -> list[str]:
    marker = log.rfind("\nfailures:\n")
    if marker < 0:
        return []
    summary = log[marker + len("\nfailures:\n") :]
    summary = summary.split("\ntest result:", maxsplit=1)[0]
    return [line[4:] for line in summary.splitlines() if line.startswith("    ")]


def run_suite(label: str, checkout: Path, log_path: Path) -> dict[str, object]:
    result = subprocess.run(
        ["cargo", "test", "-p", "finecore", "--locked"],
        cwd=checkout,
        text=True,
        encoding="utf-8",
        errors="replace",
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    log_path.write_text(result.stdout, encoding="utf-8")
    summaries = [
        {"passed": int(passed), "failed": int(failed), "ignored": int(ignored)}
        for passed, failed, ignored in SUMMARY.findall(result.stdout)
    ]
    return {
        "label": label,
        "commit": commit_for(checkout),
        "exit_code": result.returncode,
        "summaries": summaries,
        "failed_tests": failure_names(result.stdout),
        "log": str(log_path.name),
    }


def totals(run: dict[str, object]) -> dict[str, int]:
    summaries = run["summaries"]
    assert isinstance(summaries, list)
    return {
        key: sum(summary[key] for summary in summaries)
        for key in ("passed", "failed", "ignored")
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)

    baseline = run_suite(
        "PR #120 base", args.baseline, args.output / "pr120-cargo-test.log"
    )
    candidate = run_suite(
        "PR #121 candidate", args.candidate, args.output / "pr121-cargo-test.log"
    )
    baseline_failures = set(baseline["failed_tests"])
    candidate_failures = set(candidate["failed_tests"])
    report = {
        "comparison": "FineCore CLI Windows full-suite, sequential on one runner",
        "platform": platform.platform(),
        "rustc": command_version(["rustc", "--version"]),
        "cargo": command_version(["cargo", "--version"]),
        "command": "cargo test -p finecore --locked",
        "network_authentication": "not invoked; network-dependent tests remain ignored by default",
        "baseline": baseline,
        "candidate": candidate,
        "delta": {
            "candidate_only_failures": sorted(candidate_failures - baseline_failures),
            "baseline_only_failures": sorted(baseline_failures - candidate_failures),
            "same_failure_set": baseline_failures == candidate_failures,
            "baseline_totals": totals(baseline),
            "candidate_totals": totals(candidate),
        },
    }
    (args.output / "comparison.json").write_text(
        json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(json.dumps(report["delta"], indent=2, sort_keys=True))
    # Do not turn known failures into a green test result; the workflow still
    # uploads the comparison and raw logs using an `always()` artifact step.
    return 1 if baseline["exit_code"] or candidate["exit_code"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
