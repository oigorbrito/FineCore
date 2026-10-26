#!/usr/bin/env python3
"""Controlled Linux release-footprint experiments: FineCore vs pinned Orca.

The runner preserves every ordinary failure and continues through all variants.
It returns nonzero if any build, test, size, Clippy, or analysis gate fails.
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
import time
from pathlib import Path

import tomllib

ORCA_COMMIT = "2a9fd399cfbbaad773e3cbed92bb6e24d7561f01"
MCP_ARGS = ["-p", "orca-harness-tool-extensions", "--test", "mcp_stdio",
            "a_command_that_exits_immediately_fails_the_handshake", "--", "--exact", "--nocapture"]
BENCH_MARKER = "[core-tools-bench] "
VARIANTS = [
    {"name": "baseline", "release": {}, "bench": {}, "args": [], "diagnostic_only": False},
    {"name": "thin-lto", "release": {"LTO": "thin"}, "bench": {"LTO": "thin"}, "args": [], "diagnostic_only": False},
    {"name": "lto-off", "release": {"LTO": "false"}, "bench": {"LTO": "false"}, "args": [], "diagnostic_only": False},
    {"name": "opt-level-s", "release": {"OPT_LEVEL": "s"}, "bench": {"OPT_LEVEL": "s"}, "args": [], "diagnostic_only": False},
    {"name": "codegen-units-16", "release": {"CODEGEN_UNITS": "16"}, "bench": {"CODEGEN_UNITS": "16"}, "args": [], "diagnostic_only": False},
    {"name": "panic-unwind", "release": {"PANIC": "unwind"}, "bench": {"PANIC": "unwind"}, "args": [], "diagnostic_only": False},
    {"name": "unwind-tables-off", "release": {}, "bench": {}, "args": [], "rustflags": "-C force-unwind-tables=no", "diagnostic_only": True},
    {"name": "no-default-features", "release": {}, "bench": {}, "args": ["--no-default-features"], "diagnostic_only": False},
]


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def invoke(label: str, command: list[str], cwd: Path, env: dict[str, str], logs: Path,
           records: list[dict], timeout: int = 3600) -> dict:
    started = time.monotonic()
    try:
        p = subprocess.run(command, cwd=cwd, env=env, text=True, encoding="utf-8", errors="replace",
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout, check=False)
        code, output = p.returncode, p.stdout
    except subprocess.TimeoutExpired as exc:
        code, output = 124, exc.stdout or ""
        if isinstance(output, bytes):
            output = output.decode(errors="replace")
        output = f"TIMEOUT after {timeout}s\n{output}"
    except OSError as exc:
        code, output = 127, f"EXECUTION ERROR: {exc}\n"
    rec = {"label": label, "command": command, "exit_code": code,
           "elapsed_seconds": round(time.monotonic() - started, 3), "log": f"logs/{label}.log"}
    (logs / f"{label}.log").write_text(output, encoding="utf-8")
    records.append(rec)
    print(f"{label}: exit={code} elapsed={rec['elapsed_seconds']:.1f}s")
    if output:
        print("\n".join(output.rstrip().splitlines()[-4:]))
    return rec


def clean_env(source: dict[str, str]) -> dict[str, str]:
    env = source.copy()
    env["CARGO_TERM_COLOR"] = "never"
    env.pop("RUSTFLAGS", None)
    env.pop("CARGO_ENCODED_RUSTFLAGS", None)
    for key in list(env):
        if key.startswith(("CARGO_PROFILE_RELEASE_", "CARGO_PROFILE_BENCH_")):
            env.pop(key, None)
    return env


def variant_env(base: dict[str, str], variant: dict, target: Path) -> dict[str, str]:
    env = clean_env(base)
    env["CARGO_TARGET_DIR"] = str(target)
    for name, value in variant.get("release", {}).items():
        env[f"CARGO_PROFILE_RELEASE_{name}"] = value
    for name, value in variant.get("bench", {}).items():
        env[f"CARGO_PROFILE_BENCH_{name}"] = value
    if variant.get("rustflags"):
        env["RUSTFLAGS"] = variant["rustflags"]
    return env


def binary_policy(root: Path) -> dict:
    text = (root / "ci/check-binary-size.sh").read_text(encoding="utf-8")
    return {"limits_bytes": sorted({int(value) for value in re.findall(r"limit=(\d+)", text)}),
            "musl_specific": "*-linux-musl/*" in text,
            "fails_at_or_above_limit": bool(re.search(r"size\s*>=\s*limit", text))}


def git(root: Path, *args: str) -> str:
    try:
        return subprocess.check_output(["git", *args], cwd=root, text=True, stderr=subprocess.STDOUT).strip()
    except (OSError, subprocess.CalledProcessError) as exc:
        return f"ERROR: {exc}"


def benchmark_json(output: str) -> dict[str, dict]:
    data = {}
    for line in output.splitlines():
        pos = line.find(BENCH_MARKER)
        if pos < 0:
            continue
        try:
            row = json.loads(line[pos + len(BENCH_MARKER):])
            data[row["scenario"]] = row
        except (json.JSONDecodeError, KeyError):
            pass
    return data


def percentile(values: list[float], q: float) -> float:
    vals = sorted(values)
    return vals[max(0, min(len(vals) - 1, int((len(vals) - 1) * q)))]


def summarize(values: list[float], seed: int) -> dict:
    if not values:
        return {"n_pairs": 0, "median_percent": None, "ci95_percent": None}
    rng = random.Random(seed)
    medians = [statistics.median(rng.choices(values, k=len(values))) for _ in range(10000)]
    return {"n_pairs": len(values), "median_percent": round(statistics.median(values), 3),
            "ci95_percent": [round(percentile(medians, .025), 3), round(percentile(medians, .975), 3)]}


def cargo_binary(repo: str) -> str:
    return "finecore" if repo == "finecore" else "orcacode"


def cargo_test(repo: str, profile: str, args: list[str] | None = None) -> list[str]:
    result = ["cargo", "test", "--locked"]
    if profile == "release":
        result.append("--release")
    result.extend(["--workspace", *(args or [])])
    return result


def clone_binary(source: Path, destination: Path) -> Path | None:
    if not source.is_file():
        return None
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    return destination


def find_core_test(target: Path) -> Path | None:
    candidates = [p for p in (target / "release" / "deps").glob("core_tools_perf-*")
                  if p.is_file() and not p.name.endswith(".d") and os.access(p, os.X_OK)]
    return max(candidates, key=lambda p: p.stat().st_mtime_ns) if candidates else None


def collect_sections(label: str, binary: Path, root: Path, logs: Path, records: list[dict], env: dict[str, str]) -> dict:
    result = {}
    for name, command in (("size", ["size", "-A", str(binary)]),
                          ("sections", ["readelf", "-SW", str(binary)]),
                          ("dynamic", ["readelf", "-d", str(binary)]),
                          ("symbols", ["readelf", "-Ws", str(binary)])):
        rec = invoke(f"{label}-{name}", command, root, env, logs, records, 300)
        result[name] = {"exit_code": rec["exit_code"], "log": rec["log"]}
    return result


def compatibility(repo: str, variant: str, binary: Path, root: Path, env: dict[str, str],
                   logs: Path, records: list[dict]) -> list[dict]:
    commands = [[str(binary), "--version"], [str(binary), "--help"]]
    if repo == "finecore":
        commands += [[str(binary), "plugin", "--help"], [str(binary), "plugin", "validate", "--help"]]
    return [invoke(f"{repo}-{variant}-compat-{i}", command, root, env, logs, records, 60)
            for i, command in enumerate(commands)]


def run_variant(repo: str, variant: dict, root: Path, target: Path, out: Path,
                logs: Path, records: list[dict], base_env: dict[str, str], samples_mcp: int) -> dict:
    name = variant["name"]
    env = variant_env(base_env, variant, target)
    binary_name = cargo_binary(repo)
    args = variant.get("args", []) if repo == "finecore" else []
    record = {"profile": variant, "commands": [], "mcp_passed": 0}
    build = invoke(f"{repo}-{name}-release-build", ["cargo", "build", "--locked", "--release", *args, "-p", binary_name], root, env, logs, records, 5400)
    record["commands"].append(build)
    source_binary = target / "release" / binary_name
    archived = clone_binary(source_binary, out / "binaries" / f"{repo}-{name}")
    if archived:
        binary_bytes = archived.read_bytes()
        record["binary"] = {"path": str(archived), "bytes": len(binary_bytes), "sha256": digest(binary_bytes)}
        record["sections"] = collect_sections(f"{repo}-{name}", source_binary, root, logs, records, env)
        gate = invoke(f"{repo}-{name}-binary-size-gate", ["bash", "ci/check-binary-size.sh", str(source_binary)], root, env, logs, records, 60)
        record["commands"].append(gate)
        record["binary"]["size_gate_exit_code"] = gate["exit_code"]
        record["compatibility"] = compatibility(repo, name, archived, root, env, logs, records)
    else:
        record["binary"] = {"path": str(source_binary), "bytes": None, "sha256": None, "size_gate_exit_code": None}
    tests = invoke(f"{repo}-{name}-release-workspace-tests", cargo_test(repo, "release", args), root, env, logs, records, 5400)
    smoke_args = args
    smoke = invoke(f"{repo}-{name}-benchmark-smoke", ["cargo", "bench", "--locked", "--workspace", *smoke_args, "--", "--test"], root, env, logs, records, 5400)
    record["commands"].extend([tests, smoke])
    for attempt in range(1, samples_mcp + 1):
        command = ["cargo", "test", "--locked", "--release", *args, *MCP_ARGS]
        rec = invoke(f"{repo}-{name}-mcp-{attempt:02d}", command, root, env, logs, records, 600)
        record["mcp_passed"] += rec["exit_code"] == 0
        record["commands"].append(rec)
    # The workspace test above builds this real-filesystem performance harness.
    test_exe = find_core_test(target)
    if test_exe:
        archived_test = clone_binary(test_exe, out / "binaries" / f"{repo}-{name}-core_tools_perf")
        record["core_tools_perf_executable"] = str(archived_test) if archived_test else None
    else:
        record["core_tools_perf_executable"] = None

    # Repeat each build from an empty target directory to test bit-for-bit
    # reproducibility independently of Cargo's incremental artifacts.
    repeat_target = target.with_name(target.name + "-rebuild")
    shutil.rmtree(repeat_target, ignore_errors=True)
    repeat_env = variant_env(base_env, variant, repeat_target)
    repeat = invoke(f"{repo}-{name}-reproducibility-build", ["cargo", "build", "--locked", "--release", *args, "-p", binary_name], root, repeat_env, logs, records, 5400)
    repeat_path = repeat_target / "release" / binary_name
    repeat_info = {"build_exit_code": repeat["exit_code"], "elapsed_seconds": repeat["elapsed_seconds"], "bytes": None, "sha256": None}
    if repeat_path.is_file():
        repeat_bytes = repeat_path.read_bytes()
        repeat_info.update({"bytes": len(repeat_bytes), "sha256": digest(repeat_bytes)})
    record["reproducibility"] = {"repeat": repeat_info,
        "same_bytes": repeat_info["bytes"] == record["binary"].get("bytes"),
        "same_sha256": repeat_info["sha256"] == record["binary"].get("sha256")}
    shutil.rmtree(repeat_target, ignore_errors=True)
    shutil.rmtree(target, ignore_errors=True)
    return record


def direct_bench(label: str, executable: Path, root: Path, logs: Path, records: list[dict]) -> dict:
    command = [str(executable), "--nocapture", "--test-threads=1"]
    rec = invoke(label, command, root, clean_env(os.environ.copy()), logs, records, 300)
    rec["metrics"] = benchmark_json((logs / f"{label}.log").read_text(encoding="utf-8", errors="replace"))
    return rec


def pair_bench(label: str, left: Path, left_root: Path, right: Path, right_root: Path,
               samples: int, logs: Path, records: list[dict]) -> dict:
    # Warm each executable once, outside the sample set.
    direct_bench(f"{label}-warm-left", left, left_root, logs, records)
    direct_bench(f"{label}-warm-right", right, right_root, logs, records)
    sides = {"left": [], "right": []}
    raw = []
    for i in range(samples):
        order = [("left", left, left_root), ("right", right, right_root)]
        if i % 2:
            order.reverse()
        row = {}
        for side, exe, root in order:
            result = direct_bench(f"{label}-{side}-{i+1:02d}", exe, root, logs, records)
            sides[side].append(result.get("metrics", {}))
            row[side] = result.get("metrics", {})
        raw.append(row)
    scenarios = sorted(set.intersection(*(set(row) for rows in sides.values() for row in rows))) if all(sides["left"]) and all(sides["right"]) else []
    report = {}
    for scenario in scenarios:
        metrics = {}
        for metric in ("wall_seconds", "single_average_seconds"):
            deltas = [(a[scenario][metric] / b[scenario][metric] - 1) * 100
                      for a, b in zip(sides["left"], sides["right"])
                      if metric in a[scenario] and metric in b[scenario] and b[scenario][metric]]
            if deltas:
                metrics[metric] = summarize(deltas, 91 + len(label) + len(scenario) + len(metric))
        if metrics:
            report[scenario] = metrics
    return {"left": str(left), "right": str(right), "samples": samples, "metrics": report, "raw_pairs": raw}


def startup_one(label: str, binary: Path, root: Path, logs: Path) -> dict:
    rssfile = logs / f"{label}-rss.txt"
    command = ["/usr/bin/time", "-f", "%M", "-o", str(rssfile), "--", str(binary), "--version"]
    started = time.perf_counter_ns()
    try:
        p = subprocess.run(command, cwd=root, env=clean_env(os.environ.copy()), capture_output=True,
                           timeout=30, check=False)
        elapsed = (time.perf_counter_ns() - started) / 1_000_000
        rss = rssfile.read_text(encoding="utf-8").strip() if rssfile.exists() else ""
        row = {"exit_code": p.returncode, "elapsed_ms": round(elapsed, 4),
               "max_rss_kib": int(rss) if rss.isdigit() else None,
               "stdout": p.stdout.decode(errors="replace"), "stderr": p.stderr.decode(errors="replace")}
    except (OSError, subprocess.TimeoutExpired) as exc:
        row = {"exit_code": 127, "elapsed_ms": None, "max_rss_kib": None, "error": str(exc)}
    (logs / f"{label}.json").write_text(json.dumps(row, indent=2) + "\n", encoding="utf-8")
    return row


def pair_startup(label: str, left: Path, left_root: Path, right: Path, right_root: Path,
                 samples: int, logs: Path) -> dict:
    rows = []
    for i in range(samples):
        order = [("left", left, left_root), ("right", right, right_root)]
        if i % 2:
            order.reverse()
        pair = {}
        for side, binary, root in order:
            pair[side] = startup_one(f"startup-{label}-{side}-{i+1:02d}", binary, root, logs)
        rows.append(pair)
    latency = [(r["left"]["elapsed_ms"] / r["right"]["elapsed_ms"] - 1) * 100
               for r in rows if r["left"].get("elapsed_ms") and r["right"].get("elapsed_ms")]
    memory = [r["left"]["max_rss_kib"] - r["right"]["max_rss_kib"] for r in rows
              if r["left"].get("max_rss_kib") is not None and r["right"].get("max_rss_kib") is not None]
    return {"left": str(left), "right": str(right), "samples": len(rows),
            "elapsed_left_vs_right": summarize(latency, 431),
            "max_rss_kib_delta_left_minus_right_median": statistics.median(memory) if memory else None,
            "raw_pairs": rows}


def render_report(evidence: dict, logs: Path, out: Path) -> None:
    lines = [
        "# FOOTPRINT-01 controlled experiment results", "",
        f"- FineCore HEAD: `{evidence['repositories']['finecore']['head']}`",
        f"- Canonical Orca: `{evidence['repositories']['orca']['head']}`",
        f"- Environment: `{evidence['environment']['platform']}` ({evidence['environment']['machine']})",
        f"- Toolchain: {', '.join((logs / evidence['tool_versions'][key]['log']).read_text(encoding='utf-8', errors='replace').strip().splitlines()[0] for key in ('rustc', 'cargo', 'clippy') if key in evidence.get('tool_versions', {}))}",
        f"- External lock graphs identical: `{evidence['dependency_graph_match']}`; release profiles identical at baseline: `{evidence['release_profile_baseline_match']}`",
        "- Inherited size limit: `<6,700,000` bytes; unchanged. Provider calls: none.",
        "", "## Variant results", "",
        "Sizes are raw executable file bytes. Savings are relative to FineCore baseline; negative values mean growth. Repeat hash checks use a second empty target directory.",
        "",
        "| Build | Configuration | Bytes | Delta B | Savings % | Compile s (first/repeat) | Repeat identical | Release tests | Bench smoke | MCP | Size gate | Compatibility | Decision |",
        "|---|---|---:|---:|---:|---:|---|---|---|---:|---|---|---|",
    ]
    baseline = evidence["variants"].get("baseline", {}).get("finecore", {})
    base_bytes = baseline.get("binary", {}).get("bytes")
    ordered = [("finecore", "baseline", baseline), ("orca", "baseline", evidence["variants"].get("baseline", {}).get("orca", {}))]
    ordered.extend(("finecore", name, rows.get("finecore", {})) for name, rows in evidence["variants"].items() if name != "baseline")
    for repo, name, rec in ordered:
        binary = rec.get("binary", {})
        size = binary.get("bytes")
        delta = size - base_bytes if size is not None and base_bytes is not None else None
        save = (base_bytes - size) * 100 / base_bytes if size is not None and base_bytes else None
        commands = rec.get("commands", [])
        def command_exit_codes(rows: list[dict], label_part: str) -> str:
            found = [x.get("exit_code") for x in rows if label_part in x.get("label", "")]
            return ",".join(map(str, found)) if found else "n/a"
        tests = command_exit_codes(commands, "release-workspace-tests")
        smoke = command_exit_codes(commands, "benchmark-smoke")
        size_gate = command_exit_codes(commands, "binary-size-gate")
        mcp = f"{rec.get('mcp_passed', 0)}/{len([x for x in commands if '-mcp-' in x.get('label', '')])}"
        compat = rec.get("compatibility", [])
        compat_ok = f"{sum(x.get('exit_code') == 0 for x in compat)}/{len(compat)}" if compat else "n/a"
        build_s = next((x.get("elapsed_seconds") for x in commands if x.get("label", "").endswith("release-build")), None)
        repro = rec.get("reproducibility", {})
        repeat_s = repro.get("repeat", {}).get("elapsed_seconds")
        compile_s = f"{build_s}/{repeat_s}" if build_s is not None and repeat_s is not None else build_s
        same = repro.get("same_sha256")
        profile = rec.get("profile", {})
        cfg = "baseline" if name == "baseline" else ", ".join(
            [*(f"{k.removeprefix('CARGO_PROFILE_RELEASE_').lower()}={v}" for k, v in profile.get("release", {}).items()),
             *(f"{k.removeprefix('CARGO_PROFILE_BENCH_').lower()}={v}" for k, v in profile.get("bench", {}).items()),
             *( [f"RUSTFLAGS={profile['rustflags']}"] if profile.get("rustflags") else []),
             *( ["--no-default-features"] if profile.get("args") else [])]
        ) or name
        if name == "unwind-tables-off":
            decision = "Diagnostic only; reject if unwind data/debuggability is lost"
        elif name == "no-default-features":
            decision = "No-op only if feature graph matches; otherwise reject"
        elif name == "baseline":
            decision = "Reference; keep unchanged"
        else:
            decision = "Experiment only; no promotion without cross-platform review"
        lines.append(f"| {repo}/{name} | {cfg} | {size if size is not None else 'build failed'} | {delta if delta is not None else '—'} | {save:.3f} | {compile_s if compile_s is not None else '—'} | {same} | {tests} | {smoke} | {mcp} | {size_gate} | {compat_ok} | {decision} |" if save is not None else f"| {repo}/{name} | {cfg} | build failed | — | — | {compile_s} | {same} | {tests} | {smoke} | {mcp} | {size_gate} | {compat_ok} | {decision} |")

    lines += ["", "## Section attribution: baseline ELF", "", "| Section | FineCore bytes | Orca bytes | Interpretation |", "|---|---:|---:|---|"]
    section_notes = {".text": "linked executable code", ".rodata": "read-only literals/tables/assets", ".eh_frame_hdr": "unwind index", ".eh_frame": "unwind metadata", ".gcc_except_table": "language exception tables", ".rela.dyn": "dynamic relocations", ".rela.plt": "PLT relocations", ".data.rel.ro": "relocations followed by read-only protection", ".dynamic": "dynamic-link metadata", ".got": "global offset table", ".got.plt": "PLT global offset table", ".plt": "procedure linkage stubs", ".dynsym": "dynamic symbol table", ".dynstr": "dynamic symbol strings", ".data": "initialized writable data", ".bss": "zero-initialized memory; NOBITS, not file payload", ".symtab": "static symbols (expected absent after strip)", ".debug_info": "debug information (expected absent after strip)"}
    parsed = {}
    sec_re = re.compile(r"^\s*(\.[A-Za-z0-9_.$-]+)\s+(\d+)\s+")
    for repo in ("finecore", "orca"):
        path = logs / f"{repo}-baseline-size.log"
        sec = {}
        if path.exists():
            for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
                m = sec_re.match(line)
                if m:
                    sec[m.group(1)] = int(m.group(2))
        parsed[repo] = sec
    chosen = [".text", ".rodata", ".eh_frame_hdr", ".eh_frame", ".gcc_except_table", ".rela.dyn", ".rela.plt", ".data.rel.ro", ".dynamic", ".got", ".got.plt", ".plt", ".dynsym", ".dynstr", ".data", ".bss", ".symtab", ".debug_info"]
    for section in chosen:
        fsize, osize = parsed["finecore"].get(section), parsed["orca"].get(section)
        if fsize is not None or osize is not None:
            lines.append(f"| `{section}` | {fsize if fsize is not None else 'absent'} | {osize if osize is not None else 'absent'} | {section_notes.get(section, '')} |")
    lines += ["", "`size -A`, `readelf -SW/-d/-Ws`, and the stripped binary hashes are in `logs/`. ELF section totals are not identical to file bytes; NOBITS sections such as `.bss` occupy memory but not file payload. Debug/symbol audit builds are separately marked diagnostic and are never counted as candidates.",
              "", "## Dependency and symbol attribution", "",
              "`cargo-bloat` reports executable text by crate/function from an unstripped, debuginfo-enabled audit build. It does not apportion `.rodata`, relocation, or unwind bytes; LTO/inlining can blur crate attribution. Full outputs are linked below.", ""]
    for repo in ("finecore", "orca"):
        lines.append(f"### {repo}")
        lines.append("")
        for item in ("crates", "top-functions"):
            p = logs / f"{repo}-bloat-{item}.log"
            lines.append(f"- `{p.relative_to(out)}`")
            if p.exists():
                excerpt = "\n".join(p.read_text(encoding="utf-8", errors="replace").splitlines()[:24])
                lines += ["", "```text", excerpt, "```"]
        lines.append("")

    lines += ["## Paired startup, memory, and functional benchmark results", "",
              "Startup is process launch plus `--version`, timed with `/usr/bin/time`; RSS is maximum child RSS in KiB. The file-operation tests validate outputs and accuracy, not agent/model behavior. Medians and 95% paired bootstrap intervals are exploratory; multiple comparisons are not corrected.",
              "", "### Startup latency and memory", "", "| Pair (left vs right) | Startup median % | 95% CI % | Median max-RSS delta KiB | Samples |", "|---|---:|---:|---:|---:|"]
    for label, item in evidence.get("startup", {}).items():
        summary = item.get("elapsed_left_vs_right", {})
        ci = summary.get("ci95_percent")
        lines.append(f"| {label} | {summary.get('median_percent')} | {ci} | {item.get('max_rss_kib_delta_left_minus_right_median')} | {item.get('samples')} |")
    lines += ["", "### Core-tools paired workload timing", "", "Negative time deltas mean the left-hand executable was faster in the paired samples. These are local microbenchmarks, not overall agent performance.",
              "", "| Pair | Scenario | Metric | Median delta % | 95% CI % | Pairs |", "|---|---|---|---:|---:|---:|"]
    for label, pair in evidence.get("benchmarks", {}).items():
        for scenario, metrics in pair.get("metrics", {}).items():
            for metric, summary in metrics.items():
                lines.append(f"| {label} | {scenario} | {metric} | {summary.get('median_percent')} | {summary.get('ci95_percent')} | {summary.get('n_pairs')} |")
    smallest = evidence.get("summary", {}).get("minimum_functionally_passing_non_diagnostic_linux_build")
    if smallest:
        conclusion = f"Smallest non-diagnostic build that passed the scripted Linux functional checks: `{smallest['variant']}` at {smallest['bytes']:,} bytes; it remains {smallest['bytes_over_limit']:,} bytes above the inherited cap. This is not cross-platform approval."
    else:
        conclusion = "No non-diagnostic candidate passed all scripted Linux functional checks."
    lines += ["", "## Feature graph and interpretation", "", conclusion,
              f"- FineCore baseline gap to the unchanged cap: {evidence.get('summary', {}).get('baseline_bytes_over_limit')} bytes.",
              f"- `cargo tree -e features` identical with/without `--no-default-features`: `{evidence.get('feature_graphs_identical')}` (hashes and raw output in JSON/logs).",
              "- Baseline source-size, strict Clippy, and 6.7 MB binary gates are reported with their real exit codes; no threshold, lint, source-size rule, or feature set is silently changed.",
              "- All variants are Linux-only measurements. No variant is portable/production-approved until the exact candidate is built and tested on every supported target.",
              "- No profile variant was combined after the fact. A smaller but diagnostic-only build is not a safe minimum.",
              "- Complete raw records: `footprint-01-results.json`; per-command stdout/stderr in `logs/`; copied executables and test harnesses in `binaries/`.", ""]
    (out / "FOOTPRINT_01_REPORT.md").write_text("\n".join(lines), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--finecore", required=True, type=Path)
    parser.add_argument("--orca", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--samples", type=int, default=20)
    parser.add_argument("--startup-samples", type=int, default=30)
    parser.add_argument("--mcp-repeats", type=int, default=20)
    args = parser.parse_args()
    if min(args.samples, args.startup_samples, args.mcp_repeats) < 1:
        parser.error("sample counts must be positive")
    roots = {"finecore": args.finecore.resolve(), "orca": args.orca.resolve()}
    out = args.out.resolve(); logs = out / "logs"; bins = out / "binaries"
    logs.mkdir(parents=True, exist_ok=True); bins.mkdir(parents=True, exist_ok=True)
    records: list[dict] = []
    evidence: dict = {"workflow": "FOOTPRINT-01", "environment": {"platform": platform.platform(), "machine": platform.machine(),
        "python": platform.python_version(), "rustflags_initial": os.environ.get("RUSTFLAGS")},
        "repositories": {}, "variants": {}, "commands": records, "benchmarks": {}, "startup": {}, "dependency_attribution": {}, "notes": []}
    base_env = clean_env(os.environ.copy())
    for repo, root in roots.items():
        lock = tomllib.loads((root / "Cargo.lock").read_text(encoding="utf-8"))
        external = sorted((p["name"], p["version"], p.get("source"), p.get("checksum"), sorted(p.get("dependencies", [])))
                          for p in lock["package"] if "source" in p)
        cargo = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
        evidence["repositories"][repo] = {"head": git(root, "rev-parse", "HEAD"), "status": git(root, "status", "--porcelain"),
            "cargo_lock_sha256": digest((root / "Cargo.lock").read_bytes()), "external_lock_packages": len(external),
            "external_lock_graph_sha256": digest(json.dumps(external, separators=(",", ":")).encode()),
            "toolchain_sha256": digest((root / "rust-toolchain.toml").read_bytes()),
            "release_profile": cargo.get("profile", {}).get("release", {}),
            "binary": cargo_binary(repo), "binary_size_policy_sha256": digest((root / "ci/check-binary-size.sh").read_bytes()),
            "binary_size_policy": binary_policy(root),
            "source_size_script_sha256": digest((root / "ci/check-source-size.sh").read_bytes())}
    evidence["canonical_orca_pin"] = ORCA_COMMIT
    evidence["dependency_graph_match"] = evidence["repositories"]["finecore"]["external_lock_graph_sha256"] == evidence["repositories"]["orca"]["external_lock_graph_sha256"]
    evidence["toolchain_contract_match"] = evidence["repositories"]["finecore"]["toolchain_sha256"] == evidence["repositories"]["orca"]["toolchain_sha256"]
    evidence["release_profile_baseline_match"] = evidence["repositories"]["finecore"]["release_profile"] == evidence["repositories"]["orca"]["release_profile"]
    evidence["canonical_orca_exact"] = evidence["repositories"]["orca"]["head"] == ORCA_COMMIT
    cargo_roots = {name: tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8")) for name, root in roots.items()}
    evidence["workspace_dependencies_match"] = cargo_roots["finecore"].get("workspace", {}).get("dependencies", {}) == cargo_roots["orca"].get("workspace", {}).get("dependencies", {})
    evidence["workspace_members_match"] = cargo_roots["finecore"].get("workspace", {}).get("members", []) == cargo_roots["orca"].get("workspace", {}).get("members", [])
    evidence["binary_size_policy_match"] = evidence["repositories"]["finecore"]["binary_size_policy"] == evidence["repositories"]["orca"]["binary_size_policy"]
    evidence["source_size_policy_match"] = evidence["repositories"]["finecore"]["source_size_script_sha256"] == evidence["repositories"]["orca"]["source_size_script_sha256"]
    provenance_fields = ("dependency_graph_match", "toolchain_contract_match", "release_profile_baseline_match", "canonical_orca_exact", "workspace_dependencies_match", "workspace_members_match", "binary_size_policy_match", "source_size_policy_match")
    evidence["provenance_valid"] = all(evidence[field] for field in provenance_fields)
    (out / "environment.json").write_text(json.dumps({"environment": evidence["environment"], "repositories": evidence["repositories"],
        "canonical_orca_pin": ORCA_COMMIT, **{field: evidence[field] for field in provenance_fields}, "provenance_valid": evidence["provenance_valid"]}, indent=2) + "\n", encoding="utf-8")
    for command, label in ((["rustc", "--version", "--verbose"], "rustc"), (["cargo", "--version"], "cargo"), (["cargo", "clippy", "--version"], "clippy")):
        rec = invoke(f"tool-version-{label}", command, roots["finecore"], base_env, logs, records, 60)
        evidence.setdefault("tool_versions", {})[label] = {"exit_code": rec["exit_code"], "log": rec["log"]}

    # Baseline policy/test gates run on both unmodified source trees. No gate is waived.
    for repo, root in roots.items():
        target = out.parent / "targets" / f"{repo}-dev"
        shutil.rmtree(target, ignore_errors=True)
        env = variant_env(base_env, VARIANTS[0], target)
        if repo == "finecore":
            for label, command in (("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
                ("source-size", ["bash", "ci/check-source-size.sh"]),
                ("clippy", ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"])):
                invoke(f"baseline-finecore-{label}", command, root, env, logs, records)
        else:
            for label, command in (("fmt", ["cargo", "fmt", "--all", "--", "--check"]),
                ("source-size", ["bash", "ci/check-source-size.sh"]),
                ("clippy", ["cargo", "clippy", "--locked", "--workspace", "--all-targets", "--", "-D", "warnings"])):
                invoke(f"baseline-orca-{label}", command, root, env, logs, records)
        dev_test = invoke(f"baseline-{repo}-workspace-tests-dev", cargo_test(repo, "dev"), root, env, logs, records, 5400)
        evidence.setdefault("dev_tests", {})[repo] = dev_test
        shutil.rmtree(target, ignore_errors=True)

    # Exact profile baselines for FineCore and Orca, followed by isolated FineCore variants.
    artifact_bins: dict[str, dict[str, Path]] = {"finecore": {}, "orca": {}}
    artifact_tests: dict[str, dict[str, Path]] = {"finecore": {}, "orca": {}}
    variants_by_name = {v["name"]: v for v in VARIANTS}
    for repo, root in roots.items():
        target = out.parent / "targets" / f"{repo}-baseline"
        shutil.rmtree(target, ignore_errors=True)
        rec = run_variant(repo, VARIANTS[0], root, target, out, logs, records, base_env, args.mcp_repeats)
        evidence["variants"].setdefault("baseline", {})[repo] = rec
        if rec.get("binary", {}).get("path"):
            artifact_bins[repo]["baseline"] = Path(rec["binary"]["path"])
        if rec.get("core_tools_perf_executable"):
            artifact_tests[repo]["baseline"] = Path(rec["core_tools_perf_executable"])

    for variant in VARIANTS[1:]:
        target = out.parent / "targets" / f"finecore-{variant['name']}"
        shutil.rmtree(target, ignore_errors=True)
        rec = run_variant("finecore", variant, roots["finecore"], target, out, logs, records, base_env, args.mcp_repeats)
        evidence["variants"][variant["name"]] = {"finecore": rec}
        if rec.get("binary", {}).get("path"):
            artifact_bins["finecore"][variant["name"]] = Path(rec["binary"]["path"])
        if rec.get("core_tools_perf_executable"):
            artifact_tests["finecore"][variant["name"]] = Path(rec["core_tools_perf_executable"])

    base_size = evidence["variants"]["baseline"]["finecore"]["binary"].get("bytes")
    for name, entry in evidence["variants"].items():
        for repo, rec in entry.items():
            binary = rec.get("binary", {})
            size = binary.get("bytes")
            binary["delta_vs_finecore_baseline_bytes"] = size - base_size if repo == "finecore" and size is not None and base_size is not None else None
            binary["savings_percent_vs_finecore_baseline"] = round((base_size - size) * 100 / base_size, 4) if repo == "finecore" and size is not None and base_size else None
            binary["bytes_over_6700000"] = size - 6700000 if size is not None else None

    # Cargo feature graph and no-default-features negative control.
    feature_graphs = {}
    for args_extra, label in (([], "default"), (["--no-default-features"], "no-default-features")):
        env = variant_env(base_env, VARIANTS[0], out.parent / "targets" / "feature-query")
        rec = invoke(f"feature-tree-{label}", ["cargo", "tree", "--locked", "-e", "features", "-p", "finecore", *args_extra], roots["finecore"], env, logs, records, 600)
        content = (logs / f"{rec['label']}.log").read_bytes()
        feature_graphs[label] = {"exit_code": rec["exit_code"], "sha256": digest(content), "log": rec["log"]}
    evidence["feature_graphs"] = feature_graphs
    evidence["feature_graphs_identical"] = feature_graphs["default"]["sha256"] == feature_graphs["no-default-features"]["sha256"]

    # Separate diagnostic build with symbols/debug info, never included as a candidate size.
    install = invoke("install-cargo-bloat", ["cargo", "install", "cargo-bloat", "--version", "0.12.1", "--locked"], roots["finecore"], base_env, logs, records, 2400)
    evidence["cargo_bloat_install_exit_code"] = install["exit_code"]
    for repo, root in roots.items():
        target = out.parent / "targets" / f"{repo}-symbol-audit"
        shutil.rmtree(target, ignore_errors=True)
        env = clean_env(base_env); env["CARGO_TARGET_DIR"] = str(target); env["CARGO_TERM_COLOR"] = "never"
        env["CARGO_NET_OFFLINE"] = "true"
        env["CARGO_PROFILE_RELEASE_STRIP"] = "false"; env["CARGO_PROFILE_RELEASE_DEBUG"] = "1"
        build = invoke(f"{repo}-symbol-audit-build", ["cargo", "build", "--locked", "--release", "-p", cargo_binary(repo)], root, env, logs, records, 5400)
        if build["exit_code"] == 0:
            for label, command in (("crates", ["cargo", "bloat", "--release", "--crates", "-p", cargo_binary(repo)]),
                                   ("top-functions", ["cargo", "bloat", "--release", "-n", "100", "-p", cargo_binary(repo)])):
                invoke(f"{repo}-bloat-{label}", command, root, env, logs, records, 1800)
            binary = target / "release" / cargo_binary(repo)
            symbol_output = invoke(f"{repo}-symbol-audit-nm", ["nm", "-S", "--size-sort", "--demangle", str(binary)], root, env, logs, records, 300)
            evidence.setdefault("symbol_audit", {})[repo] = {"build_exit_code": build["exit_code"], "nm_exit_code": symbol_output["exit_code"], "is_diagnostic_only": True}
        shutil.rmtree(target, ignore_errors=True)

    # Paired end-to-end CLI startup/RSS and tool-level workload timing. This
    # is exploratory, not a performance acceptance test or a provider call.
    startup = {}
    for label, left_repo, left_name, right_repo, right_name in [("baseline-finecore-vs-orca", "finecore", "baseline", "orca", "baseline")]:
        if left_name in artifact_bins[left_repo] and right_name in artifact_bins[right_repo]:
            startup[label] = pair_startup(label, artifact_bins[left_repo][left_name], roots[left_repo],
                artifact_bins[right_repo][right_name], roots[right_repo], args.startup_samples, logs)
    for name in variants_by_name:
        if name == "baseline" or name not in artifact_bins["finecore"] or "baseline" not in artifact_bins["finecore"]:
            continue
        label = f"finecore-{name}-vs-baseline"
        startup[label] = pair_startup(label, artifact_bins["finecore"][name], roots["finecore"],
            artifact_bins["finecore"]["baseline"], roots["finecore"], args.startup_samples, logs)
    evidence["startup"] = startup

    benchmarks = {}
    pairs = [("finecore-baseline-vs-orca", "finecore", "baseline", "orca", "baseline")]
    pairs.extend((f"finecore-{name}-vs-baseline", "finecore", name, "finecore", "baseline")
                 for name in variants_by_name if name != "baseline")
    for label, left_repo, left_name, right_repo, right_name in pairs:
        if left_name not in artifact_tests[left_repo] or right_name not in artifact_tests[right_repo]:
            benchmarks[label] = {"status": "skipped: benchmark executable unavailable"}
            continue
        benchmarks[label] = pair_bench(label, artifact_tests[left_repo][left_name], roots[left_repo],
            artifact_tests[right_repo][right_name], roots[right_repo], args.samples, logs, records)
    evidence["benchmarks"] = benchmarks

    all_failed = [{"label": rec["label"], "exit_code": rec["exit_code"]} for rec in records if rec["exit_code"] != 0]
    evidence["failed_commands"] = all_failed
    tested_candidates = [(base_size, "baseline")] if base_size is not None else []
    for variant_name, variant_rows in evidence["variants"].items():
        rec = variant_rows.get("finecore", {})
        if variant_name == "baseline" or rec.get("profile", {}).get("diagnostic_only"):
            continue
        if variant_name == "no-default-features" and not evidence.get("feature_graphs_identical"):
            continue
        commands_by_label = {row["label"]: row["exit_code"] for row in rec.get("commands", [])}
        functional_ok = (rec.get("binary", {}).get("bytes") is not None and rec.get("reproducibility", {}).get("same_sha256") is True
            and commands_by_label.get(f"finecore-{variant_name}-release-build") == 0
            and commands_by_label.get(f"finecore-{variant_name}-release-workspace-tests") == 0
            and commands_by_label.get(f"finecore-{variant_name}-benchmark-smoke") == 0
            and rec.get("mcp_passed") == args.mcp_repeats
            and all(row.get("exit_code") == 0 for row in rec.get("compatibility", [])))
        if functional_ok:
            tested_candidates.append((rec["binary"]["bytes"], variant_name))
    smallest = min(tested_candidates) if tested_candidates else None
    evidence["summary"] = {"variant_count_including_baseline": len(VARIANTS), "binary_size_limit_bytes_exclusive": 6700000,
        "baseline_bytes": base_size, "baseline_bytes_over_limit": base_size - 6700000 if base_size is not None else None,
        "minimum_functionally_passing_non_diagnostic_linux_build": {"variant": smallest[1], "bytes": smallest[0],
            "bytes_over_limit": smallest[0] - 6700000} if smallest else None,
        "binary_size_gate_changed": False, "provider_calls": 0, "all_failures_preserved": True,
        "provenance_valid": evidence["provenance_valid"]}
    (out / "footprint-01-results.json").write_text(json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    render_report(evidence, logs, out)
    print(f"Evidence written to {out}; failures recorded={len(all_failed)}")
    return 1 if all_failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
