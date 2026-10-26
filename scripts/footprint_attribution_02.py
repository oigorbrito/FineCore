#!/usr/bin/env python3
"""Run only equivalent FineCore/Orca symbol-attribution builds.

Dependencies are fetched and checksum-verified before cargo-bloat is invoked.
This runner deliberately performs no tests, benchmarks, or profile variants.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

import tomllib

ORCA_COMMIT = "2a9fd399cfbbaad773e3cbed92bb6e24d7561f01"
FINECORE_SOURCE_BASE = "066229b4eb35f061e837197ebd105710fb42c80b"
LOCK_GRAPH_SHA256 = "d5c31c7e8e50cdf6269db3c5bfbe93595f8f298370313664b443b2aa642b7661"
TOOLCHAIN_SHA256 = "f32338bdcf61d8100b7125baa6a4822dfb41c81c8299489df0475436c6218f7a"
LOCK_SHA256 = {
    "finecore": "50c11948814512f9596e1c5f2d4bea697bb8ef3ac00b5d3946d0ec0ba0906350",
    "orca": "3b62985bc71d27506864018058ef3d3eeaf5e9acd339eaa86b801ab114c8df6e",
}
BUMPALO_CHECKSUM = "72f5acc6cb2ba439de613abc23857ec3d78374d8ed5ac84e9d11336e87da8649"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def atomic_json(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    tmp.replace(path)


def tracked_file_sha256(root: Path, relative_path: str) -> str:
    data = subprocess.check_output(["git", "show", f"HEAD:{relative_path}"], cwd=root)
    return sha256(data)


def lock_data(root: Path) -> tuple[dict, list[tuple], str]:
    lock_path = root / "Cargo.lock"
    lock = tomllib.loads(lock_path.read_text(encoding="utf-8"))
    external = sorted(
        (p["name"], p["version"], p.get("source"), p.get("checksum"),
         sorted(p.get("dependencies", [])))
        for p in lock["package"] if "source" in p
    )
    graph_hash = sha256(json.dumps(external, separators=(",", ":")).encode())
    return lock, external, graph_hash


def run(label: str, command: list[str], cwd: Path, env: dict[str, str],
        logs: Path, records: list[dict], timeout: int = 7200) -> dict:
    log_path = logs / f"{label}.log"
    logs.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    code = 127
    timed_out = False
    print(f"[attribution] START {label}: {' '.join(command)}", flush=True)
    try:
        with log_path.open("w", encoding="utf-8", errors="replace") as log:
            proc = subprocess.Popen(command, cwd=cwd, env=env, text=True, encoding="utf-8",
                                    errors="replace", stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, bufsize=1)

            def drain() -> None:
                assert proc.stdout is not None
                prefixes = ("Compiling ", "Checking ", "Finished ", "Running ", "Downloading ",
                            "Installing ", "Installed ", "error:", "warning:")
                for line in proc.stdout:
                    log.write(line)
                    log.flush()
                    if line.lstrip().startswith(prefixes):
                        sys.stdout.write(line)
                        sys.stdout.flush()

            reader = threading.Thread(target=drain, daemon=True)
            reader.start()
            try:
                code = proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
                code = 124
                timed_out = True
            reader.join()
            if timed_out:
                log.write(f"\nTIMEOUT after {timeout}s\n")
                log.flush()
            if proc.stdout is not None:
                proc.stdout.close()
    except OSError as exc:
        with log_path.open("a", encoding="utf-8", errors="replace") as log:
            log.write(f"EXECUTION ERROR: {exc}\n")
    rec = {"label": label, "command": command, "cwd": str(cwd), "exit_code": code,
           "elapsed_seconds": round(time.monotonic() - started, 3), "log": f"logs/{label}.log"}
    records.append(rec)
    print(f"[attribution] END {label}: exit={code} elapsed={rec['elapsed_seconds']:.1f}s", flush=True)
    return rec


def verify_registry_cache(external: list[tuple], cargo_home: Path) -> tuple[list[dict], list[dict]]:
    verified, missing = [], []
    cache_root = cargo_home / "registry" / "cache"
    for name, version, source, checksum, _dependencies in external:
        if not source or not source.startswith("registry+") or not checksum:
            missing.append({"name": name, "version": version, "reason": "unsupported/non-checksummed source"})
            continue
        archives = list(cache_root.glob(f"*/{name}-{version}.crate"))
        matching = next((archive for archive in archives
                         if sha256(archive.read_bytes()) == checksum), None)
        if matching:
            verified.append({"name": name, "version": version, "checksum": checksum})
        else:
            observed = [sha256(a.read_bytes()) for a in archives]
            missing.append({"name": name, "version": version, "expected_checksum": checksum,
                            "observed_checksums": observed, "archive_found": bool(archives)})
    return verified, missing


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--finecore", required=True, type=Path)
    parser.add_argument("--orca", required=True, type=Path)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--cargo-home", type=Path,
                        default=Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")))
    args = parser.parse_args()
    finecore, orca = args.finecore.resolve(), args.orca.resolve()
    out = args.out.resolve()
    logs = out / "logs"
    out.mkdir(parents=True, exist_ok=True)
    cargo_home = args.cargo_home.resolve()
    base_env = os.environ.copy()
    base_env.update({"CARGO_HOME": str(cargo_home), "CARGO_TERM_COLOR": "never",
                     "RUSTUP_TOOLCHAIN": "1.99.1"})
    records: list[dict] = []
    evidence = {"experiment": "FOOTPRINT-ATTRIBUTION-02", "finecore_head": None,
                "orca_head": None, "environment": {"platform": platform.platform(),
                "machine": platform.machine(), "python": platform.python_version()},
                "dependency_verification": {}, "diagnostic_builds": {}, "commands": records,
                "analysis_scope": "diagnostic builds and attribution only; no tests or benchmarks"}
    atomic_json(out / "footprint-attribution-02-partial.json", evidence)

    roots = {"finecore": finecore, "orca": orca}
    locks = {}
    for repo, root in roots.items():
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
        status = subprocess.check_output(["git", "status", "--porcelain"], cwd=root, text=True).strip()
        evidence[f"{repo}_head"] = head
        if status:
            raise RuntimeError(f"{repo} checkout is not clean: {status}")
        lock, external, graph_hash = lock_data(root)
        locks[repo] = (lock, external, graph_hash)
        lock_hash = tracked_file_sha256(root, "Cargo.lock")
        toolchain_hash = tracked_file_sha256(root, "rust-toolchain.toml")
        profile = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8")).get("profile", {}).get("release", {})
        evidence.setdefault("repositories", {})[repo] = {
            "head": head, "status": status, "cargo_lock_sha256": lock_hash,
            "external_packages": len(external), "external_lock_graph_sha256": graph_hash,
            "toolchain_sha256": toolchain_hash, "release_profile": profile,
        }
        if lock_hash != LOCK_SHA256[repo] or graph_hash != LOCK_GRAPH_SHA256 or toolchain_hash != TOOLCHAIN_SHA256:
            raise RuntimeError(f"{repo} lock/toolchain provenance differs from FOOTPRINT-01")
        bumpalo = [p for p in external if p[0] == "bumpalo" and p[1] == "3.20.3"]
        if len(bumpalo) != 1 or bumpalo[0][3] != BUMPALO_CHECKSUM:
            raise RuntimeError(f"{repo} does not lock the expected bumpalo 3.20.3 checksum")

    if locks["finecore"][2] != locks["orca"][2]:
        raise RuntimeError("FineCore and Orca external lock graphs are not identical")
    if evidence["orca_head"] != ORCA_COMMIT:
        raise RuntimeError(f"Orca HEAD must be pinned commit {ORCA_COMMIT}")
    ancestry = subprocess.run(["git", "merge-base", "--is-ancestor", FINECORE_SOURCE_BASE, "HEAD"],
                              cwd=finecore, check=False)
    if ancestry.returncode != 0:
        raise RuntimeError("FineCore checkout is not descended from the FOOTPRINT-01 source head")
    source_diff = subprocess.run(["git", "diff", "--quiet", FINECORE_SOURCE_BASE, "HEAD", "--",
                                  "Cargo.toml", "Cargo.lock", "crates"], cwd=finecore, check=False)
    if source_diff.returncode != 0:
        raise RuntimeError("FineCore product source/manifest differs from FOOTPRINT-01 HEAD")
    if evidence["repositories"]["finecore"]["release_profile"] != evidence["repositories"]["orca"]["release_profile"]:
        raise RuntimeError("baseline release profiles differ")
    atomic_json(out / "footprint-attribution-02-partial.json", evidence)

    for command, label in ((["rustc", "--version", "--verbose"], "toolchain-rustc"),
                           (["cargo", "--version"], "toolchain-cargo")):
        run(label, command, finecore, base_env, logs, records, 60)

    # Resolve lockfile packages before analysis, then verify every archive checksum.
    for repo, root in roots.items():
        run(f"{repo}-cargo-fetch-locked", ["cargo", "fetch", "--locked"], root,
            {**base_env, "CARGO_NET_OFFLINE": "false"}, logs, records, 3600)
    failed_fetches = [r for r in records if r["label"].endswith("cargo-fetch-locked") and r["exit_code"] != 0]
    if failed_fetches:
        evidence["dependency_verification"] = {"status": "failed cargo fetch", "failures": failed_fetches}
        atomic_json(out / "footprint-attribution-02-partial.json", evidence)
        return 1

    cache_report = {}
    for repo, (_lock, external, graph_hash) in locks.items():
        verified, missing = verify_registry_cache(external, cargo_home)
        cache_report[repo] = {"lock_graph_sha256": graph_hash, "locked_package_count": len(external),
                              "verified_archive_count": len(verified), "verified_packages": verified,
                              "missing_or_mismatched": missing,
                              "bumpalo_3_20_3": next((x for x in verified if x["name"] == "bumpalo"), None)}
    evidence["dependency_verification"] = cache_report
    atomic_json(out / "dependency-verification.json", cache_report)
    atomic_json(out / "footprint-attribution-02-partial.json", evidence)
    if any(row["missing_or_mismatched"] for row in cache_report.values()):
        print("[attribution] Refusing to call cargo-bloat: locked registry archives are not all verified.", flush=True)
        return 1

    install = run("install-cargo-bloat-0.12.1",
                  ["cargo", "install", "cargo-bloat", "--version", "0.12.1", "--locked"],
                  finecore, {**base_env, "CARGO_NET_OFFLINE": "false"}, logs, records, 3600)
    if install["exit_code"] != 0:
        atomic_json(out / "footprint-attribution-02-partial.json", evidence)
        return 1
    version = run("cargo-bloat-version", ["cargo", "bloat", "--version"], finecore,
                  {**base_env, "CARGO_NET_OFFLINE": "true"}, logs, records, 60)
    if version["exit_code"] != 0:
        atomic_json(out / "footprint-attribution-02-partial.json", evidence)
        return 1

    target_root = Path(os.environ.get("RUNNER_TEMP", "/tmp")) / "footprint-attribution-02-targets"
    for repo, root in roots.items():
        package = "finecore" if repo == "finecore" else "orcacode"
        target = target_root / repo
        shutil.rmtree(target, ignore_errors=True)
        env = {**base_env, "CARGO_NET_OFFLINE": "true", "CARGO_TARGET_DIR": str(target),
               "CARGO_PROFILE_RELEASE_STRIP": "false", "CARGO_PROFILE_RELEASE_DEBUG": "1"}
        build = run(f"{repo}-diagnostic-release-build",
                    ["cargo", "build", "--locked", "--release", "-p", package],
                    root, env, logs, records, 7200)
        binary = target / "release" / package
        build_record = {"package": package, "build_exit_code": build["exit_code"],
                        "diagnostic_profile": {"release_strip": False, "release_debug": 1,
                                               "inherited_release_profile": evidence["repositories"][repo]["release_profile"]},
                        "binary_present": binary.is_file(), "binary_bytes": None, "binary_sha256": None,
                        "cargo_bloat_crates_exit_code": None, "cargo_bloat_top100_exit_code": None}
        evidence["diagnostic_builds"][repo] = build_record
        if build["exit_code"] == 0 and binary.is_file():
            data = binary.read_bytes()
            build_record["binary_bytes"] = len(data)
            build_record["binary_sha256"] = sha256(data)
            for label, command in (
                ("size", ["size", "-A", str(binary)]),
                ("sections", ["readelf", "-SW", str(binary)]),
                ("symbols", ["nm", "-S", "--size-sort", "--demangle", str(binary)]),
                ("bloat-crates", ["cargo", "bloat", "--release", "--crates", "-p", package]),
                ("bloat-top100", ["cargo", "bloat", "--release", "-n", "100", "-p", package]),
            ):
                result = run(f"{repo}-{label}", command, root, env, logs, records, 3600)
                if label == "bloat-crates":
                    build_record["cargo_bloat_crates_exit_code"] = result["exit_code"]
                elif label == "bloat-top100":
                    build_record["cargo_bloat_top100_exit_code"] = result["exit_code"]
        atomic_json(out / "footprint-attribution-02-partial.json", evidence)

    evidence["failed_commands"] = [
        {"label": row["label"], "exit_code": row["exit_code"]} for row in records if row["exit_code"] != 0
    ]
    evidence["complete"] = all(
        record["build_exit_code"] == 0 and record["binary_present"]
        and record["cargo_bloat_crates_exit_code"] == 0
        and record["cargo_bloat_top100_exit_code"] == 0
        for record in evidence["diagnostic_builds"].values()
    )
    atomic_json(out / "footprint-attribution-02.json", evidence)
    atomic_json(out / "footprint-attribution-02-partial.json", evidence)
    return 0 if evidence["complete"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
