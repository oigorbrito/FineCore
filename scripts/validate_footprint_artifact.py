#!/usr/bin/env python3
"""Read-only verification of the frozen FOOTPRINT-01 Actions artifact.

No rebuilds, model calls, or changes to the original evidence.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

ORIGINAL_FINECORE = "066229b4eb35f061e837197ebd105710fb42c80b"
ORIGINAL_ORCA = "2a9fd399cfbbaad773e3cbed92bb6e24d7561f01"
EXPECTED_BYTES = {
    ("finecore", "baseline"): 9_334_616,
    ("orca", "baseline"): 9_342_840,
    ("finecore", "thin-lto"): 10_841_752,
    ("finecore", "lto-off"): 11_054_328,
    ("finecore", "opt-level-s"): 9_960_504,
    ("finecore", "codegen-units-16"): 9_955_032,
    ("finecore", "panic-unwind"): 10_452_952,
    ("finecore", "unwind-tables-off"): 9_334_616,
    ("finecore", "no-default-features"): 9_334_616,
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify(evidence_root: Path) -> dict:
    source = evidence_root / "footprint-01-results.json"
    evidence = json.loads(source.read_text(encoding="utf-8"))
    assert evidence["repositories"]["finecore"]["head"] == ORIGINAL_FINECORE
    assert evidence["repositories"]["orca"]["head"] == ORIGINAL_ORCA
    assert evidence.get("provenance_valid") is True
    assert evidence.get("dependency_graph_match") is True
    assert evidence.get("release_profile_baseline_match") is True
    assert evidence["summary"]["binary_size_limit_bytes_exclusive"] == 6_700_000
    checked = []
    for (repo, variant), size in EXPECTED_BYTES.items():
        record = evidence["variants"][variant][repo]
        binary = evidence_root / "binaries" / f"{repo}-{variant}"
        assert binary.is_file(), f"missing binary: {binary}"
        assert binary.stat().st_size == size, f"binary size mismatch: {binary}"
        assert record["binary"]["bytes"] == size, f"JSON size mismatch: {binary}"
        actual_sha = sha256_file(binary)
        assert actual_sha == record["binary"]["sha256"], f"hash mismatch: {binary}"
        assert record["reproducibility"]["same_bytes"] is True
        assert record["reproducibility"]["same_sha256"] is True
        assert record["reproducibility"]["repeat"]["bytes"] == size
        assert record["reproducibility"]["repeat"]["sha256"] == actual_sha
        assert record["mcp_passed"] == 20
        commands = {row["label"]: row["exit_code"] for row in record["commands"]}
        assert commands[f"{repo}-{variant}-release-build"] == 0
        assert commands[f"{repo}-{variant}-release-workspace-tests"] == 0
        assert commands[f"{repo}-{variant}-benchmark-smoke"] == 0
        assert record["binary"]["size_gate_exit_code"] != 0, "inherited size failure must remain visible"
        checked.append(f"{repo}/{variant}")
    assert evidence["summary"]["baseline_bytes"] == EXPECTED_BYTES[("finecore", "baseline")]
    assert evidence["summary"]["baseline_bytes_over_limit"] == 2_634_616
    assert len(evidence["startup"]) == 8, "missing paired startup/RSS comparisons"
    assert len(evidence["benchmarks"]) == 8, "missing paired core-tools comparisons"
    commands = evidence["commands"]
    assert len(commands) == 653, f"expected 653 commands, got {len(commands)}"
    for command in commands:
        reference = Path(command["log"])
        assert not reference.is_absolute() and ".." not in reference.parts, reference
        log_path = evidence_root / reference
        assert log_path.is_file(), f"missing command log: {log_path}"
    assert evidence["feature_graphs_identical"] is True
    return {
        "original_run": 40351250854,
        "binaries_verified": checked,
        "command_logs_verified": len(commands),
        "paired_startup_comparisons": len(evidence["startup"]),
        "paired_workload_comparisons": len(evidence["benchmarks"]),
        "smallest_verified_bytes": EXPECTED_BYTES[("finecore", "baseline")],
        "bytes_above_exclusive_cap": 2_634_616,
        "code_or_build_executed": False,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("evidence_root", type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.evidence_root.resolve()), indent=2))


if __name__ == "__main__":
    main()
