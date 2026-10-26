#!/usr/bin/env python3
"""Apply the overall BENCH-02 verdict after all independent suites finish."""
import argparse
import json
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    if not args.manifest.is_file():
        print(f"BENCH-02 FAIL: no manifest at {args.manifest}")
        return 1
    data = json.loads(args.manifest.read_text(encoding="utf-8"))
    statuses = data.get("statuses", {})
    failures = []
    if not data.get("donor_pin_valid"):
        failures.append("donor commit was not the required pin")
    if not data.get("benchmark_source_hashes", {}).get("identical"):
        failures.append("probed tool and benchmark sources differ between trees")
    build_inputs = data.get("build_inputs", {})
    if not build_inputs.get("rust_toolchain_identical"):
        failures.append("rust-toolchain.toml differs between trees")
    if not build_inputs.get("normalized_lock_graph_identical"):
        failures.append("locked dependency graphs differ beyond FineCore/Orca CLI identity")
    if statuses.get("paired_core_tools_valid") is not True:
        failures.append("paired core-tools sample set incomplete")
    required = {
        "reporting_tests",
        *(f"core_tools_accuracy_{side}" for side in ("finecore", "orca")),
        *(f"core_tools_build_{side}" for side in ("finecore", "orca")),
        *(f"core_tools_budget_{side}" for side in ("finecore", "orca")),
        *(f"mcp_{suite}_{side}" for side in ("finecore", "orca")
          for suite in ("tests", "accuracy", "distribution", "pareto")),
        *(f"subagent_{suite}_{side}" for side in ("finecore", "orca")
          for suite in ("concurrency", "simulation")),
    }
    for name in sorted(required - statuses.keys()):
        failures.append(f"required suite did not record a status: {name}")
    for name, status in sorted(statuses.items()):
        if name == "paired_core_tools_valid":
            continue
        ok = status == 0
        print(f"{'PASS' if ok else 'FAIL'} {name}: {status}")
        if not ok:
            failures.append(f"{name} returned {status}")
    print("\nBENCH-02 overall: " + ("FAIL" if failures else "PASS"))
    for failure in failures:
        print(f"- {failure}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
