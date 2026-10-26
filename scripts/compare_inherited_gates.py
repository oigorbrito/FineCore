#!/usr/bin/env python3
"""Run FineCore and canonical Orca gates sequentially on one runner.

The runner intentionally records every gate and continues after ordinary
failures. Its nonzero exit is the aggregate result; it never converts an
inherited failure into a pass.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import random
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
import tomllib
from pathlib import Path

ORCA_COMMIT = "2a9fd399cfbbaad773e3cbed92bb6e24d7561f01"
BENCH_CMD = [
    "cargo",
    "test",
    "--locked",
    "--release",
    "-q",
    "-p",
    "orca-harness-tools",
    "--test",
    "core_tools_perf",
    "--",
    "--nocapture",
    "--test-threads=1",
]
SOURCE_FINDING = re.compile(r"^(crates/.+?): (\d+) lines \(maximum 599;")
BENCH_LINE = "[core-tools-bench] "
ANSI_ESCAPE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
FOCUSED_EXIT_CMD = [
    "cargo",
    "test",
    "--locked",
    "-p",
    "orca-harness-tool-extensions",
    "--test",
    "mcp_stdio",
    "a_command_that_exits_immediately_fails_the_handshake",
    "--",
    "--exact",
    "--nocapture",
]


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def capture(command: list[str], cwd: Path, env: dict[str, str]) -> tuple[int, str]:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            text=True,
            encoding="utf-8",
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=3600,
            check=False,
        )
        return result.returncode, result.stdout
    except subprocess.TimeoutExpired as exc:
        output = exc.stdout or ""
        if isinstance(output, bytes):
            output = output.decode(errors="replace")
        return 124, f"TIMEOUT after 3600 seconds\n{output}"
    except OSError as exc:
        return 127, f"EXECUTION ERROR: {exc}\n"


def binary_size_policy(root: Path) -> dict:
    text = (root / "ci/check-binary-size.sh").read_text(encoding="utf-8")
    return {
        "limits_bytes": sorted(int(value) for value in re.findall(r"limit=(\d+)", text)),
        "musl_specific_limit": "*-linux-musl/*" in text,
        "fails_at_or_above_limit": bool(re.search(r"size\s*>=\s*limit", text)),
    }


def external_lock_graph(root: Path) -> tuple[str, int]:
    lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
    packages = []
    for package in lock["package"]:
        if "source" not in package:
            continue
        packages.append(
            {
                "name": package["name"],
                "version": package["version"],
                "source": package["source"],
                "checksum": package.get("checksum"),
                "dependencies": sorted(package.get("dependencies", [])),
            }
        )
    packages.sort(key=lambda p: (p["name"], p["version"], p["source"]))
    encoded = json.dumps(packages, sort_keys=True, separators=(",", ":")).encode()
    return sha256(encoded), len(packages)


def git_value(root: Path, *args: str) -> str:
    code, output = capture(["git", *args], root, os.environ.copy())
    return output.strip() if code == 0 else f"ERROR({code}): {output.strip()}"


def record_gate(
    label: str,
    command: list[str],
    root: Path,
    env: dict[str, str],
    log_dir: Path,
    results: list[dict],
) -> tuple[int, str]:
    started = time.monotonic()
    code, output = capture(command, root, env)
    elapsed = round(time.monotonic() - started, 3)
    (log_dir / f"{label}.log").write_text(output, encoding="utf-8")
    result = {
        "gate": label,
        "command": command,
        "exit_code": code,
        "elapsed_seconds": elapsed,
        "log": f"logs/{label}.log",
    }
    results.append(result)
    print(f"{label}: exit={code} elapsed={elapsed:.1f}s")
    if output:
        print("\n".join(output.rstrip().splitlines()[-8:]))
    return code, output


def source_findings(output: str) -> dict[str, int]:
    findings = {}
    for line in output.splitlines():
        match = SOURCE_FINDING.match(line)
        if match:
            findings[match.group(1)] = int(match.group(2))
    return findings


def benchmark_metrics(output: str) -> dict[str, dict]:
    metrics = {}
    for line in output.splitlines():
        marker = line.find(BENCH_LINE)
        if marker < 0:
            continue
        try:
            item = json.loads(line[marker + len(BENCH_LINE) :])
        except json.JSONDecodeError:
            continue
        metrics[item["scenario"]] = item
    return metrics


def percentile(values: list[float], fraction: float) -> float:
    values = sorted(values)
    index = max(0, min(len(values) - 1, int((len(values) - 1) * fraction)))
    return values[index]


def bootstrap_summary(values: list[float], seed: int) -> dict:
    if not values:
        return {"n_pairs": 0, "median_change_percent": None, "ci95_percent": None}
    rng = random.Random(seed)
    medians = [statistics.median(rng.choices(values, k=len(values))) for _ in range(10000)]
    return {
        "n_pairs": len(values),
        "median_change_percent": round(statistics.median(values), 3),
        "ci95_percent": [round(percentile(medians, 0.025), 3), round(percentile(medians, 0.975), 3)],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--finecore", type=Path, required=True)
    parser.add_argument("--orca", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--samples", type=int, default=20)
    args = parser.parse_args()
    roots = {"finecore": args.finecore.resolve(), "orca": args.orca.resolve()}
    out = args.out.resolve()
    logs = out / "logs"
    logs.mkdir(parents=True, exist_ok=True)

    metadata = {
        "platform": platform.platform(),
        "machine": platform.machine(),
        "python": platform.python_version(),
        "repositories": {},
        "environment": {
            key: os.environ.get(key)
            for key in (
                "RUSTFLAGS",
                "CARGO_ENCODED_RUSTFLAGS",
                "CARGO_PROFILE_RELEASE_LTO",
                "CARGO_PROFILE_RELEASE_CODEGEN_UNITS",
                "CARGO_PROFILE_RELEASE_OPT_LEVEL",
                "CARGO_PROFILE_RELEASE_STRIP",
                "CARGO_PROFILE_RELEASE_PANIC",
            )
        },
    }
    for name, root in roots.items():
        cargo = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
        metadata["repositories"][name] = {
            "head": git_value(root, "rev-parse", "HEAD"),
            "status_porcelain": git_value(root, "status", "--porcelain"),
            "rust_toolchain_sha256": sha256((root / "rust-toolchain.toml").read_bytes()),
            "cargo_lock_sha256": sha256((root / "Cargo.lock").read_bytes()),
            "external_lock_graph_sha256": external_lock_graph(root)[0],
            "external_lock_packages": external_lock_graph(root)[1],
            "release_profile": cargo.get("profile", {}).get("release", {}),
            "workspace_dependencies": cargo.get("workspace", {}).get("dependencies", {}),
            "workspace_members": cargo.get("workspace", {}).get("members", []),
            "source_size_script_sha256": sha256((root / "ci/check-source-size.sh").read_bytes()),
            "binary_size_script_sha256": sha256((root / "ci/check-binary-size.sh").read_bytes()),
            "binary_size_policy": binary_size_policy(root),
            "release_binary": "finecore" if name == "finecore" else "orcacode",
        }
    metadata["same_external_dependency_graph"] = (
        metadata["repositories"]["finecore"]["external_lock_graph_sha256"]
        == metadata["repositories"]["orca"]["external_lock_graph_sha256"]
    )
    metadata["same_workspace_dependencies"] = (
        metadata["repositories"]["finecore"]["workspace_dependencies"]
        == metadata["repositories"]["orca"]["workspace_dependencies"]
    )
    metadata["same_workspace_members"] = (
        metadata["repositories"]["finecore"]["workspace_members"]
        == metadata["repositories"]["orca"]["workspace_members"]
    )
    metadata["same_rust_toolchain_contract"] = (
        metadata["repositories"]["finecore"]["rust_toolchain_sha256"]
        == metadata["repositories"]["orca"]["rust_toolchain_sha256"]
    )
    metadata["same_source_size_gate"] = (
        metadata["repositories"]["finecore"]["source_size_script_sha256"]
        == metadata["repositories"]["orca"]["source_size_script_sha256"]
    )
    metadata["same_binary_size_policy"] = (
        metadata["repositories"]["finecore"]["binary_size_policy"]
        == metadata["repositories"]["orca"]["binary_size_policy"]
    )
    metadata["same_release_profile"] = (
        metadata["repositories"]["finecore"]["release_profile"]
        == metadata["repositories"]["orca"]["release_profile"]
    )
    metadata["orca_commit_is_canonical"] = (
        metadata["repositories"]["orca"]["head"] == ORCA_COMMIT
    )
    (out / "environment.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")

    versions = {}
    for command, label in ((["rustc", "--version", "--verbose"], "rustc"), (["cargo", "--version"], "cargo"), (["cargo", "clippy", "--version"], "clippy")):
        code, output = capture(command, roots["finecore"], os.environ.copy())
        versions[label] = {"exit_code": code, "output": output.strip()}
    (out / "tool-versions.json").write_text(json.dumps(versions, indent=2) + "\n", encoding="utf-8")

    all_results = {}
    target_root = Path(tempfile.mkdtemp(prefix="inherited-gates-targets-")).resolve()
    for name, root in roots.items():
        target = (target_root / name).resolve()
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(target)
        env["CARGO_TERM_COLOR"] = "never"
        results: list[dict] = []
        gates = [
            ("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
            ("source-size", ["bash", "ci/check-source-size.sh"]),
            ("clippy", ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"]),
            ("workspace-tests", ["cargo", "test", "--locked", "--workspace"]),
            ("benchmark-smoke", ["cargo", "bench", "--locked", "--workspace", "--", "--test"]),
        ]
        for label, command in gates:
            record_gate(f"{name}-{label}", command, root, env, logs, results)

        focused_passes = 0
        for attempt in range(1, 21):
            code, _ = record_gate(
                f"{name}-mcp-immediate-exit-{attempt:02d}",
                FOCUSED_EXIT_CMD,
                root,
                env,
                logs,
                results,
            )
            focused_passes += code == 0
        results.append(
            {
                "gate": f"{name}-mcp-immediate-exit-aggregate",
                "command": FOCUSED_EXIT_CMD,
                "exit_code": 0 if focused_passes == 20 else 1,
                "attempts": 20,
                "passed": focused_passes,
            }
        )
        print(f"{name}-mcp-immediate-exit-aggregate: {focused_passes}/20 passed")

        binary = "finecore" if name == "finecore" else "orcacode"
        build_code, _ = record_gate(
            f"{name}-release-build",
            ["cargo", "build", "--locked", "--release", "-p", binary],
            root,
            env,
            logs,
            results,
        )
        binary_path = target / "release" / binary
        if build_code == 0 and binary_path.is_file():
            data = binary_path.read_bytes()
            measurements = {
                "path": str(binary_path),
                "bytes": len(data),
                "sha256": sha256(data),
            }
            size_code, _ = record_gate(
                f"{name}-binary-size",
                ["bash", "ci/check-binary-size.sh", str(binary_path)],
                root,
                env,
                logs,
                results,
            )
            measurements["gate_exit_code"] = size_code
            try:
                section_report = subprocess.run(
                    ["size", "-A", str(binary_path)],
                    text=True,
                    encoding="utf-8",
                    errors="replace",
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    check=False,
                ).stdout
            except OSError as exc:
                section_report = f"size tool unavailable: {exc}\n"
            (out / f"{name}-binary-size-sections.txt").write_text(
                section_report,
                encoding="utf-8",
            )
        else:
            measurements = {"path": str(binary_path), "bytes": None, "sha256": None, "gate_exit_code": None}
        all_results[name] = {"gates": results, "binary": measurements}

    comparison = {}
    source_results = {}
    for name in roots:
        source_log = (logs / f"{name}-source-size.log").read_text(encoding="utf-8", errors="replace")
        findings = source_findings(source_log)
        source_results[name] = {
            path: {
                "lines": count,
                "sha256": sha256((roots[name] / path).read_bytes()),
            }
            for path, count in findings.items()
        }
    all_source_paths = sorted(set(source_results["finecore"]) | set(source_results["orca"]))
    comparison["source_size"] = {
        "finecore_violations": source_results["finecore"],
        "orca_violations": source_results["orca"],
        "same_findings_and_line_counts": {
            path: (
                path in source_results["finecore"]
                and path in source_results["orca"]
                and source_results["finecore"][path]["lines"] == source_results["orca"][path]["lines"]
            )
            for path in all_source_paths
        },
        "same_file_content": {
            path: (
                path in source_results["finecore"]
                and path in source_results["orca"]
                and source_results["finecore"][path]["sha256"] == source_results["orca"][path]["sha256"]
            )
            for path in all_source_paths
        },
    }
    clippy_counts = {}
    for name in roots:
        text = (logs / f"{name}-clippy.log").read_text(encoding="utf-8", errors="replace")
        text = ANSI_ESCAPE.sub("", text)
        clippy_counts[name] = {
            "result_large_err_diagnostics": text.count("error: the `Err`-variant returned"),
            "lint": "clippy::result_large_err" if "clippy::result_large_err" in text else None,
        }
    comparison["clippy"] = {
        "finecore": clippy_counts["finecore"],
        "orca": clippy_counts["orca"],
        "same_diagnostic_count": clippy_counts["finecore"] == clippy_counts["orca"],
    }
    comparison["binary_bytes_delta_finecore_minus_orca"] = None
    fine_bytes = all_results["finecore"]["binary"]["bytes"]
    orca_bytes = all_results["orca"]["binary"]["bytes"]
    if fine_bytes is not None and orca_bytes is not None:
        comparison["binary_bytes_delta_finecore_minus_orca"] = fine_bytes - orca_bytes

    # Warm both test binaries, then collect alternating paired samples. The
    # bootstrap interval is exploratory: it is not a performance claim.
    bench = {name: [] for name in roots}
    for name, root in roots.items():
        target = (target_root / name).resolve()
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(target)
        env["CARGO_TERM_COLOR"] = "never"
        record_gate(f"{name}-benchmark-warmup", BENCH_CMD, root, env, logs, all_results[name]["gates"])
    for sample in range(1, args.samples + 1):
        order = ["finecore", "orca"] if sample % 2 else ["orca", "finecore"]
        for name in order:
            root = roots[name]
            env = os.environ.copy()
            env["CARGO_TARGET_DIR"] = str((target_root / name).resolve())
            env["CARGO_TERM_COLOR"] = "never"
            label = f"benchmark-{name}-{sample:02d}"
            code, output = record_gate(label, BENCH_CMD, root, env, logs, all_results[name]["gates"])
            if code == 0:
                bench[name].append(benchmark_metrics(output))
    benchmark_results = {}
    scenarios = sorted(set.intersection(*(set(item) for rows in bench.values() for item in rows))) if all(bench.values()) else []
    for scenario in scenarios:
        metrics = {}
        for metric in ("wall_seconds", "single_average_seconds"):
            pairs = []
            for fine_sample, orca_sample in zip(bench["finecore"], bench["orca"]):
                if metric in fine_sample[scenario] and metric in orca_sample[scenario]:
                    baseline = orca_sample[scenario][metric]
                    if baseline:
                        pairs.append((fine_sample[scenario][metric] / baseline - 1) * 100)
            if pairs:
                metrics[metric] = bootstrap_summary(pairs, 112 + len(scenario) + len(metric))
        if metrics:
            benchmark_results[scenario] = metrics
    comparison["mcp_immediate_exit_repeats"] = {
        name: {
            "attempts": 20,
            "passed": sum(
                gate.get("exit_code") == 0
                for gate in all_results[name]["gates"]
                if gate["gate"].startswith(f"{name}-mcp-immediate-exit-")
                and not gate["gate"].endswith("-aggregate")
            ),
        }
        for name in roots
    }
    comparison["benchmark"] = {
        "samples_requested_per_harness": args.samples,
        "samples_completed": {name: len(bench[name]) for name in roots},
        "finecore_vs_orca_percent_change": benchmark_results,
        "interpretation": "Exploratory paired microbenchmark; no speedup claim. Same runner; no provider calls.",
    }
    (out / "comparison.json").write_text(json.dumps(comparison, indent=2) + "\n", encoding="utf-8")

    summary = {
        "metadata": metadata,
        "repositories": all_results,
        "comparison_artifacts": ["comparison.json", "environment.json", "tool-versions.json"],
        "ordinary_gate_failures_are_preserved": True,
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
    shutil.rmtree(target_root, ignore_errors=True)
    provenance_checks = (
        "same_external_dependency_graph",
        "same_workspace_dependencies",
        "same_workspace_members",
        "same_release_profile",
        "same_rust_toolchain_contract",
        "same_source_size_gate",
        "same_binary_size_policy",
        "orca_commit_is_canonical",
    )
    failed = [
        f"provenance/{key}=false"
        for key in provenance_checks
        if not metadata[key]
    ]
    failed.extend(
        f"{name}/{gate['gate']}={gate['exit_code']}"
        for name, data in all_results.items()
        for gate in data["gates"]
        if gate["exit_code"] != 0
    )
    print(f"Evidence written to {out}")
    if failed:
        print("FAIL (preserved): " + ", ".join(failed))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
