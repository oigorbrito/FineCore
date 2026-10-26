#!/usr/bin/env python3
"""Paired empirical comparison, with no assumed positive outcome.

This is fault-injection evidence, not a model-quality or speed benchmark.
"""
import json
import statistics
import sys
from pathlib import Path

CASES = (
    "healthy",
    "transport_once",
    "transport_twice",
    "incomplete_once",
    "authentication",
    "pretool_transport_once",
    "pretool_incomplete_once",
    "pretool_authentication",
)
REPETITIONS = 20


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
    for (case, _trial), row in observations.items():
        before = case.startswith("pretool_")
        if row["phase"] != ("before_tool" if before else "after_tool"):
            raise ValueError(f"wrong failure phase: {row}")
        attempts = row["stage_attempts"]
        if not (1 <= attempts <= 2):
            raise ValueError(f"invalid attempt count: {row}")
        if expected_arm == "baseline" and attempts != 1:
            raise ValueError(f"unexpected baseline replay: {row}")
        if row["tool_effects"] != (0 if before else attempts):
            raise ValueError(f"inconsistent recorded external effects: {row}")
        if row["model_generations"] != attempts * (1 if before else 2):
            raise ValueError(f"inconsistent number of model calls: {row}")
    return observations


def summarize(baseline: Path, retry: Path):
    arms = {
        "baseline": read_rows(baseline, "baseline"),
        "retry": read_rows(retry, "retry"),
    }
    table = {}
    for case in CASES:
        table[case] = {}
        for arm in arms:
            rows = [arms[arm][(case, trial)] for trial in range(REPETITIONS)]
            failures = sorted({r["error_line"] for r in rows if r["error_line"]})
            table[case][arm] = {
                "successes": sum(r["success"] for r in rows),
                "trials": len(rows),
                "tool_effects_total": sum(r["tool_effects"] for r in rows),
                "model_generations_total": sum(r["model_generations"] for r in rows),
                "trials_with_duplicate_effects": sum(r["tool_effects"] > 1 for r in rows),
                "median_elapsed_micros_diagnostic_only": statistics.median(
                    r["elapsed_micros"] for r in rows
                ),
                "terminal_error_examples": failures[:3],
            }
        base = table[case]["baseline"]
        candidate = table[case]["retry"]
        table[case]["delta"] = {
            "additional_successes_per_20": candidate["successes"] - base["successes"],
            "additional_model_generations_per_20": (
                candidate["model_generations_total"] - base["model_generations_total"]
            ),
            "additional_external_effects_per_20": (
                candidate["tool_effects_total"] - base["tool_effects_total"]
            ),
            "additional_trials_with_duplicated_effects": (
                candidate["trials_with_duplicate_effects"]
                - base["trials_with_duplicate_effects"]
            ),
        }
    return {
        "design": "20 deterministic fault trials per case and arm; actual append-only tool effects",
        "baseline_head": "2f980015a733ec449ca1ea8be51e97da08220fb1",
        "candidate_head": "092ff1ff0fb654c814663a296318abc06266584f",
        "limitations": [
            "No provider calls, real coding tasks, tokens or US dollar cost",
            "Synthetic deterministic fault distribution; no population effectiveness claim",
            "Latency is diagnostic only and not evidence of a speedup",
            "The same fixture and toolchain are run on a single Ubuntu runner",
        ],
        "results": table,
    }


def main():
    if len(sys.argv) != 4:
        raise SystemExit("usage: analyze_upstream_effect_01.py BASE_LOG RETRY_LOG OUTPUT_JSON")
    report = summarize(Path(sys.argv[1]), Path(sys.argv[2]))
    output = Path(sys.argv[3])
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
