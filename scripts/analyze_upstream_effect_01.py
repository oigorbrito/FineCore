#!/usr/bin/env python3
"""Fail-closed comparison of two runs of the same real-tool fault fixture.

This is a controlled recovery experiment, NOT a benchmark of model quality,
provider spend, latency improvements, or broad programming effectiveness.
"""
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

CASES = {
    "healthy": ((True, 1), (True, 1)),
    "transport_once": ((False, 1), (True, 2)),
    "transport_twice": ((False, 1), (False, 2)),
    "incomplete_once": ((False, 1), (True, 2)),
    "authentication": ((False, 1), (False, 1)),
}
REPETITIONS = 20
ARMS = ("baseline", "retry")


def read_rows(path: Path, expected_arm: str):
    observations = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        prefix = "UPSTREAM_EFFECT_JSON "
        if prefix not in line:
            continue
        row = json.loads(line.split(prefix, 1)[1])
        if row["arm"] != expected_arm:
            raise ValueError("wrong arm: " + str(row))
        key = (row["case"], row["trial"])
        if key in observations:
            raise ValueError("duplicate sample: " + str(key))
        observations[key] = row
    expected = {(name, trial) for name in CASES for trial in range(REPETITIONS)}
    if set(observations) != expected:
        raise ValueError(
            f"{expected_arm}: expected {len(expected)} rows, "
            f"missing={sorted(expected-set(observations))}, "
            f"extra={sorted(set(observations)-expected)}"
        )
    return observations


def summarize(base: Path, candidate: Path):
    arms = {
        "baseline": read_rows(base, "baseline"),
        "retry": read_rows(candidate, "retry"),
    }
    table = {}
    for case, targets in CASES.items():
        table[case] = {}
        for arm, (success_target, attempts_target) in zip(ARMS, targets):
            rows = [arms[arm][(case, trial)] for trial in range(REPETITIONS)]
            for row in rows:
                if row["success"] != success_target:
                    raise ValueError(f"{arm}/{case} unexpected success: {row}")
                if row["stage_attempts"] != attempts_target:
                    raise ValueError(f"{arm}/{case} unexpected attempts: {row}")
                if row["tool_effects"] != attempts_target:
                    raise ValueError(f"{arm}/{case} unexpected external effects: {row}")
                if row["model_generations"] != 2 * attempts_target:
                    raise ValueError(f"{arm}/{case} unexpected model invocations: {row}")
            table[case][arm] = {
                "successes": sum(row["success"] for row in rows),
                "trials": len(rows),
                "tool_effects_total": sum(row["tool_effects"] for row in rows),
                "model_generations_total": sum(row["model_generations"] for row in rows),
                "trials_with_duplicate_effects": sum(row["tool_effects"] > 1 for row in rows),
                "median_elapsed_micros_diagnostic_only": statistics.median(
                    row["elapsed_micros"] for row in rows
                ),
            }
    return {
        "evidence": "20 deterministic faults per case per arm; real append-only tool effect",
        "baseline": "FineCore integration 2f980015a733ec449ca1ea8be51e97da08220fb1",
        "candidate": "FineCore retry #110 092ff1ff0fb654c814663a296318abc06266584f",
        "limitations": [
            "no provider calls, no real coding tasks, no token/USD measurements",
            "model and fault distribution are deliberately controlled",
            "time samples are diagnostic, not performance attribution",
            "workflows run on one Ubuntu runner with pinned source and compiler",
        ],
        "results": table,
    }


def main():
    if len(sys.argv) != 4:
        raise SystemExit("usage: analyze_upstream_effect_01.py BASE_LOG RETRY_LOG OUTPUT_JSON")
    summary = summarize(Path(sys.argv[1]), Path(sys.argv[2]))
    target = Path(sys.argv[3])
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
