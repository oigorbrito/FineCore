#!/usr/bin/env python3
"""Run a paired, no-provider FineCore/Orca benchmark on one Linux worker.

This runner keeps each suite independent, alternates candidate order for each
core-tools pair, and stores only this run's evidence in an external output dir.
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import os
import platform
import random
import shutil
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import tomllib

DONOR_COMMIT = "2a9fd399cfbbaad773e3cbed92bb6e24d7561f01"
SIDES = ("finecore", "orca")


def command(args: list[str], cwd: Path, timeout: int = 900) -> dict[str, Any]:
    started = time.perf_counter()
    try:
        load_before = os.getloadavg()
    except (AttributeError, OSError):
        load_before = None
    try:
        result = subprocess.run(
            args, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            text=True, timeout=timeout, check=False,
        )
        output, code, timed_out = result.stdout, result.returncode, False
    except subprocess.TimeoutExpired as exc:
        output = exc.stdout or ""
        if isinstance(output, bytes):
            output = output.decode(errors="replace")
        code, timed_out = 124, True
    except OSError as exc:
        output, code, timed_out = f"{type(exc).__name__}: {exc}\\n", 127, False
    elapsed = time.perf_counter() - started
    try:
        load_after = os.getloadavg()
    except (AttributeError, OSError):
        load_after = None
    return {"command": args, "cwd": str(cwd), "exit_code": code,
            "timed_out": timed_out, "elapsed_seconds": elapsed,
            "load_average_before": load_before, "load_average_after": load_after,
            "output": output}


def save_command(path: Path, result: dict[str, Any]) -> None:
    path.mkdir(parents=True, exist_ok=True)
    (path / "command.json").write_text(json.dumps(
        {k: v for k, v in result.items() if k != "output"}, indent=2) + "\n")
    (path / "output.log").write_text(result["output"], encoding="utf-8")


def git_value(repo: Path, *args: str) -> str:
    result = subprocess.run(["git", *args], cwd=repo, text=True,
                            capture_output=True, check=True)
    return result.stdout.strip()


def tree_hash(root: Path, paths: tuple[str, ...]) -> tuple[str, int]:
    records = subprocess.run(
        ["git", "ls-files", "--stage", "-z", "--", *paths], cwd=root,
        capture_output=True, check=True,
    ).stdout.split(b"\0")
    digest = hashlib.sha256()
    count = 0
    for record in records:
        if not record:
            continue
        metadata, raw_path = record.split(b"\t", 1)
        mode = metadata.split(b" ", 1)[0]
        path = root / raw_path.decode()
        relative = path.relative_to(root).as_posix()
        digest.update(mode + b"\0" + relative.encode() + b"\0")
        digest.update(path.read_bytes())
        count += 1
    return digest.hexdigest(), count


def normalized_lock_hash(root: Path) -> str:
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    packages = []
    for package in lock.get("package", []):
        name = package["name"]
        if name in {"finecore", "orcacode"}:
            name = "<host-cli-identity>"
        packages.append({
            "name": name,
            "version": package["version"],
            "source": package.get("source"),
            "checksum": package.get("checksum"),
            "dependencies": sorted(package.get("dependencies", [])),
        })
    canonical = json.dumps(sorted(packages, key=lambda item: (item["name"], item["version"], item["source"] or "")),
                           separators=(",", ":"), sort_keys=True)
    return hashlib.sha256(canonical.encode()).hexdigest()


def cpu_metadata() -> dict[str, Any]:
    cpu = "unknown"
    try:
        for line in Path("/proc/cpuinfo").read_text().splitlines():
            if line.lower().startswith("model name"):
                cpu = line.split(":", 1)[1].strip()
                break
    except OSError:
        pass
    try:
        memory = next(line for line in Path("/proc/meminfo").read_text().splitlines()
                      if line.startswith("MemTotal:"))
    except (OSError, StopIteration):
        memory = None
    try:
        load = os.getloadavg()
    except (AttributeError, OSError):
        load = None
    return {"platform": platform.platform(), "machine": platform.machine(),
            "logical_cpus": os.cpu_count(), "cpu_model": cpu,
            "memory_total": memory, "load_average_at_start": load,
            "runner_os": os.environ.get("RUNNER_OS"),
            "runner_arch": os.environ.get("RUNNER_ARCH"),
            "image_os": os.environ.get("ImageOS"),
            "image_version": os.environ.get("ImageVersion")}


def percentile(values: list[float], fraction: float) -> float:
    values = sorted(values)
    return values[min(len(values) - 1, max(0, math.ceil(len(values) * fraction) - 1))]


def bootstrap_median_ci(differences: list[float], seed: int) -> list[float] | None:
    if len(differences) < 2:
        return None
    rng = random.Random(seed)
    medians = [statistics.median(rng.choices(differences, k=len(differences)))
               for _ in range(10_000)]
    return [percentile(medians, .025), percentile(medians, .975)]


def load_report_module(candidate: Path):
    path = candidate / "benchmarks/core-tools/report.py"
    spec = importlib.util.spec_from_file_location("bench02_core_report", path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load core-tools reporter: {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def parse_core_tools_output(module, output: str) -> dict[tuple[str, str], list[dict]]:
    """Normalize Rust test-harness prefixes without changing donor report.py."""
    records = []
    for raw in output.splitlines():
        prefix, separator, payload = raw.partition(module.PREFIX)
        if not separator:
            continue
        if prefix.strip(". ") and not (
            prefix.startswith("test ") and prefix.rstrip().endswith(" ...")
        ):
            raise ValueError(f"unexpected benchmark marker context: {prefix!r}")
        records.append(module.PREFIX + payload)
    return module.parse("\n".join(records))


def collect_core_tools(candidate: Path, donor: Path, out: Path, samples: int,
                       statuses: dict[str, Any]) -> None:
    executables: dict[str, Path] = {}
    for side, repo in (("finecore", candidate), ("orca", donor)):
        accuracy = command(["cargo", "test", "--locked", "-p", "orca-harness-tools",
                            "--test", "core_tools"], repo)
        save_command(out / "core-tools" / side / "accuracy", accuracy)
        statuses[f"core_tools_accuracy_{side}"] = accuracy["exit_code"]

        build = command(["cargo", "test", "--locked", "--release", "-p",
                         "orca-harness-tools", "--test", "core_tools_perf", "--no-run",
                         "--message-format=json"], repo, timeout=1800)
        save_command(out / "core-tools" / side / "build", build)
        statuses[f"core_tools_build_{side}"] = build["exit_code"]
        if build["exit_code"] != 0:
            continue
        executable = None
        for line in build["output"].splitlines():
            try:
                item = json.loads(line)
            except json.JSONDecodeError:
                continue
            target = item.get("target", {})
            if (item.get("reason") == "compiler-artifact"
                    and target.get("name") == "core_tools_perf"
                    and "test" in target.get("kind", [])
                    and item.get("executable")):
                executable = Path(item["executable"])
        if executable is None:
            statuses[f"core_tools_build_{side}"] = 1
            save_command(out / "core-tools" / side / "build-error", {
                "command": [], "cwd": str(repo), "exit_code": 1,
                "timed_out": False, "elapsed_seconds": 0,
                "output": "cargo emitted no core_tools_perf test executable\n"})
        else:
            executables[side] = executable

    module = load_report_module(candidate)
    raw: dict[str, list[str]] = {side: [] for side in SIDES}
    parsed_by_pair: dict[int, dict[str, dict[tuple[str, str], list[dict]]]] = {}
    seed = 20261026
    rng = random.Random(seed)
    for pair in range(1, samples + 1):
        order = list(SIDES) if pair % 2 else list(reversed(SIDES))
        if pair == 1 and rng.randrange(2):
            order.reverse()
        parsed_by_pair[pair] = {}
        for side in order:
            if side not in executables:
                continue
            result = command([str(executables[side]), "--nocapture", "--test-threads=1"],
                             candidate if side == "finecore" else donor, timeout=300)
            sample_dir = out / "core-tools" / "samples" / side / f"pair-{pair:02d}"
            save_command(sample_dir, result)
            raw[side].append(result["output"])
            parsed_by_pair[pair][side] = parse_core_tools_output(module, result["output"])
            statuses[f"core_tools_sample_{side}_{pair:02d}"] = result["exit_code"]

    summaries: dict[str, dict[str, Any]] = {}
    try:
        for side in SIDES:
            parsed: dict[tuple[str, str], list[dict]] = {}
            for text in raw[side]:
                for key, values in parse_core_tools_output(module, text).items():
                    parsed.setdefault(key, []).extend(values)
            measurements, summary = module.aggregate(parsed, samples)
            suffixes = (
                " wall p50", " wall p99", " throughput p50",
                " single average p50", " speedup median",
            )
            for entry in summary:
                name = entry["name"]
                scenario = name.split("/")[-1]
                suffix = next((value for value in suffixes if scenario.endswith(value)), None)
                if suffix is None:
                    continue
                scenario = scenario.removesuffix(suffix)
                tool = name.split("/")[1]
                matching = parsed.get((tool, scenario), [])
                if not matching:
                    continue
                if suffix == " throughput p50":
                    values = [item["operations"] / item["wall_seconds"] for item in matching]
                elif suffix == " single average p50":
                    values = [item["single_average_seconds"] for item in matching]
                elif suffix == " speedup median":
                    values = [item["speedup"] for item in matching]
                else:
                    values = [item["wall_seconds"] for item in matching]
                entry["range"] = f"min={min(values):.9g}, max={max(values):.9g}; n={len(values)}"
            side_dir = out / "core-tools" / side / "report"
            side_dir.mkdir(parents=True, exist_ok=True)
            report = {"schema": 1, "generated_at": datetime.now(timezone.utc).isoformat(),
                      "git_sha": git_value(candidate if side == "finecore" else donor,
                                           "rev-parse", "HEAD"),
                      "platform": platform.platform(), "machine": platform.machine(),
                      "samples": samples, "measurements": measurements}
            (side_dir / "measurements.json").write_text(json.dumps(report, indent=2) + "\n")
            (side_dir / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
            (side_dir / "raw.txt").write_text("\n".join(raw[side]), encoding="utf-8")
            budget = command([sys.executable, str(candidate / "benchmarks/shared/check_budgets.py"),
                              "--suite", "core-tools", "--dir", str(side_dir)], candidate)
            save_command(side_dir / "budget", budget)
            statuses[f"core_tools_budget_{side}"] = budget["exit_code"]
            summaries[side] = {"measurements": measurements, "summary": summary}
    except (ValueError, KeyError) as exc:
        statuses["core_tools_reports"] = f"invalid/incomplete samples: {exc}"

    if len(summaries) != 2:
        statuses["paired_core_tools_valid"] = False
        return

    metrics = []
    names = sorted({(item["tool"], item["scenario"])
                    for item in summaries["finecore"]["measurements"]})
    for tool, scenario in names:
        candidate_values: list[float] = []
        donor_values: list[float] = []
        paired: list[dict[str, Any]] = []
        for pair in range(1, samples + 1):
            pair_data = parsed_by_pair.get(pair, {})
            c = pair_data.get("finecore", {}).get((tool, scenario), [])
            d = pair_data.get("orca", {}).get((tool, scenario), [])
            if len(c) == 1 and len(d) == 1:
                cv, dv = c[0]["wall_seconds"], d[0]["wall_seconds"]
                candidate_values.append(cv)
                donor_values.append(dv)
                paired.append({"pair": pair, "finecore_seconds": cv,
                               "orca_seconds": dv, "delta_seconds": cv - dv,
                               "order": "finecore_first" if pair % 2 else "orca_first"})
        differences = [item["delta_seconds"] for item in paired]
        metrics.append({"tool": tool, "scenario": scenario,
                        "valid_pairs": len(paired), "paired": paired,
                        "finecore_median_seconds": statistics.median(candidate_values) if candidate_values else None,
                        "orca_median_seconds": statistics.median(donor_values) if donor_values else None,
                        "median_paired_delta_seconds": statistics.median(differences) if differences else None,
                        "median_delta_bootstrap_95pct_ci_seconds": bootstrap_median_ci(differences, seed + len(metrics))})
    paired_doc = {"seed": seed, "samples_per_side": samples,
                  "order": "alternating FineCore-first / Orca-first; initial order seeded",
                  "bootstrap": "10,000 paired-resample medians, percentile interval",
                  "metrics": metrics}
    (out / "core-tools").mkdir(parents=True, exist_ok=True)
    (out / "core-tools/paired-comparison.json").write_text(json.dumps(paired_doc, indent=2) + "\n")
    statuses["paired_core_tools_valid"] = all(item["valid_pairs"] == samples for item in metrics)


def run_independent_suites(repo: Path, side: str, out: Path,
                           statuses: dict[str, Any]) -> None:
    # MCP tests and reports are separate processes so one failure cannot hide
    # later evidence. These fixtures are deterministic and make no network calls.
    mcp_dir = out / "mcp" / side
    mcp_dir.mkdir(parents=True, exist_ok=True)
    commands = [
        ("tests", [sys.executable, "-m", "unittest", "discover", "-s",
                    "benchmarks/mcp", "-p", "*_test.py", "-v"]),
        ("accuracy", [sys.executable, "benchmarks/mcp/accuracy.py"]),
        ("distribution", [sys.executable, "benchmarks/mcp/distribution.py"]),
        ("pareto", [sys.executable, "benchmarks/mcp/pareto.py"]),
    ]
    for name, args in commands:
        result = command(args, repo)
        save_command(mcp_dir / name, result)
        statuses[f"mcp_{name}_{side}"] = result["exit_code"]

    for name, args in (("concurrency", ["bash", "benchmarks/subagent/run.sh", "--quick"]),
                       ("simulation", ["bash", "benchmarks/subagent/sim.sh", "--quick"])):
        result = command(args, repo, timeout=2400)
        dest = out / "subagent" / side / name
        save_command(dest, result)
        generated = repo / "benchmarks/results/subagent"
        if generated.exists():
            shutil.copytree(generated, dest / "generated", dirs_exist_ok=True)
        statuses[f"subagent_{name}_{side}"] = result["exit_code"]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--donor", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=20)
    args = parser.parse_args()
    if args.samples < 2:
        parser.error("--samples must be at least 2 for paired intervals")
    candidate, donor, out = args.candidate.resolve(), args.donor.resolve(), args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    source_paths = ("crates/tools", "benchmarks/core-tools", "benchmarks/mcp", "benchmarks/subagent")
    hashes = {side: tree_hash(repo, source_paths) for side, repo in
              (("finecore", candidate), ("orca", donor))}
    file_hashes = {}
    for side, repo in (("finecore", candidate), ("orca", donor)):
        file_hashes[side] = {}
        for name in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
            file_hashes[side][name] = hashlib.sha256((repo / name).read_bytes()).hexdigest()
    rustc = subprocess.run(["rustc", "--version", "-v"], text=True,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, check=False)
    manifest: dict[str, Any] = {
        "schema": 1, "started_at": datetime.now(timezone.utc).isoformat(),
        "finecore_commit": git_value(candidate, "rev-parse", "HEAD"),
        "orca_commit": git_value(donor, "rev-parse", "HEAD"),
        "required_orca_commit": DONOR_COMMIT,
        "donor_pin_valid": git_value(donor, "rev-parse", "HEAD") == DONOR_COMMIT,
        "environment": cpu_metadata(), "rustc": rustc.stdout,
        "cargo": subprocess.run(["cargo", "--version"], text=True,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                 check=False).stdout.strip(),
        "benchmark_source_hashes": {
            "paths": list(source_paths),
            "finecore": {"sha256": hashes["finecore"][0], "files": hashes["finecore"][1]},
            "orca": {"sha256": hashes["orca"][0], "files": hashes["orca"][1]},
            "identical": hashes["finecore"][0] == hashes["orca"][0]},
        "build_inputs": {"sha256": file_hashes,
                         "rust_toolchain_identical": file_hashes["finecore"]["rust-toolchain.toml"] == file_hashes["orca"]["rust-toolchain.toml"],
                         "cargo_lock_identical": file_hashes["finecore"]["Cargo.lock"] == file_hashes["orca"]["Cargo.lock"],
                         "normalized_lock_graph_sha256": {
                             "finecore": normalized_lock_hash(candidate),
                             "orca": normalized_lock_hash(donor),
                         },
                         "normalized_lock_graph_identical": normalized_lock_hash(candidate) == normalized_lock_hash(donor)},
        "samples_per_candidate": args.samples,
        "statuses": {},
        "scope": "core-tools paired; MCP and subagent suites run independently on both trees; no model/API calls",
    }
    statuses = manifest["statuses"]
    if not manifest["donor_pin_valid"]:
        statuses["donor_pin"] = "FAIL: donor checkout does not match pinned commit"
    try:
        reporting = command(["bash", "benchmarks/run_reporting_tests.sh"], candidate)
        save_command(out / "reporting-tests", reporting)
        statuses["reporting_tests"] = reporting["exit_code"]
    except Exception as exc:  # noqa: BLE001 - continue to collect later suites
        statuses["reporting_tests"] = f"runner error: {type(exc).__name__}: {exc}"
    try:
        collect_core_tools(candidate, donor, out, args.samples, statuses)
    except Exception as exc:  # noqa: BLE001 - independent suites must still run
        statuses["core_tools_runner"] = f"runner error: {type(exc).__name__}: {exc}"
        statuses["paired_core_tools_valid"] = False
    for side, repo in (("finecore", candidate), ("orca", donor)):
        try:
            run_independent_suites(repo, side, out, statuses)
        except Exception as exc:  # noqa: BLE001 - preserve the other tree's results
            statuses[f"suite_runner_{side}"] = f"runner error: {type(exc).__name__}: {exc}"
    manifest["finished_at"] = datetime.now(timezone.utc).isoformat()
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(json.dumps({"results": str(out), "statuses": statuses}, indent=2))
    # Deliberately leave the workflow step green; finalization applies the
    # consolidated verdict only after every suite has run and artifacts exist.
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
