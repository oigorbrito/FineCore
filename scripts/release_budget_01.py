#!/usr/bin/env python3
"""Measure reproducible single-binary archive references from existing builds.

This utility does not build software or publish artifacts. Inputs are verified
against recorded SHA-256 digests before deterministic archive bytes are made.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import sys
import tarfile
import zipfile
import zlib
from pathlib import Path
from typing import Any

METHOD = "normalized single-file USTAR+gzip or ZIP_DEFLATED archive; level 6"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def validate_member(member: str) -> None:
    if not member or member in {".", ".."} or "/" in member or "\\" in member:
        raise ValueError(f"archive member must be one root-level filename: {member!r}")


def make_archive(data: bytes, archive_format: str, member: str) -> bytes:
    """Return normalized archive bytes matching the release's one-file layout."""
    validate_member(member)
    output = io.BytesIO()
    if archive_format == "tar.gz":
        with (
            gzip.GzipFile(fileobj=output, mode="wb", filename="", mtime=0, compresslevel=6) as gz,
            tarfile.open(fileobj=gz, mode="w", format=tarfile.USTAR_FORMAT) as archive,
        ):
            info = tarfile.TarInfo(member)
            info.size = len(data)
            info.mode = 0o755
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            info.mtime = 0
            archive.addfile(info, io.BytesIO(data))
    elif archive_format == "zip":
        with zipfile.ZipFile(output, mode="w", compression=zipfile.ZIP_DEFLATED,
                             compresslevel=6) as archive:
            info = zipfile.ZipInfo(member, date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, data, compress_type=zipfile.ZIP_DEFLATED, compresslevel=6)
    else:
        raise ValueError(f"unsupported archive format: {archive_format!r}")
    return output.getvalue()


def measure(row: dict[str, Any]) -> dict[str, Any]:
    required = {"label", "path", "sha256", "format", "member"}
    missing = required - row.keys()
    if missing:
        raise ValueError(f"input missing fields: {', '.join(sorted(missing))}")
    binary_path = Path(row["path"])
    data = binary_path.read_bytes()
    actual_sha = sha256(data)
    if actual_sha != row["sha256"].lower():
        raise ValueError(f"{row['label']}: input SHA-256 mismatch ({actual_sha})")
    archive = make_archive(data, row["format"], row["member"])
    return {
        "label": row["label"],
        "format": row["format"],
        "member": row["member"],
        "binary_bytes": len(data),
        "binary_sha256": actual_sha,
        "archive_bytes": len(archive),
        "archive_sha256": sha256(archive),
        "compressed_to_binary_ratio": round(len(archive) / len(data), 6),
    }


def render(manifest_path: Path) -> dict[str, Any]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    inputs = manifest.get("inputs")
    if not isinstance(inputs, list) or not inputs:
        raise ValueError("manifest must contain a non-empty 'inputs' array")
    labels = [row.get("label") for row in inputs]
    if len(labels) != len(set(labels)):
        raise ValueError("input labels must be unique")
    return {
        "method": METHOD,
        "python": sys.version.split()[0],
        "zlib": zlib.ZLIB_VERSION,
        "rows": [measure(row) for row in inputs],
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("manifest", type=Path, help="JSON with binary paths and expected SHA-256 values")
    args = parser.parse_args()
    try:
        print(json.dumps(render(args.manifest), indent=2, sort_keys=True))
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        print(f"release-budget-01: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
