#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
exec python3 "$repo_root/scripts/audit_crates.py" \
  --root "$repo_root" \
  --baseline "$repo_root/ci/baselines/orca-d89bf274.json"
