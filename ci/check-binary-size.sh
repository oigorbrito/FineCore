#!/usr/bin/env bash
set -euo pipefail

baseline_mode=0
if [[ "${1:-}" == "--baseline-orca-d89" ]]; then
  baseline_mode=1
  shift
fi
binary="${1:-target/release/orcacode}"
# Static musl builds carry their own libc and allocator, about 2 MB more
# than the macOS binary; 0.6.2 shipped at 8.62 MB.
case "$binary" in
  *-linux-musl/*) limit=9000000 ;;
  *) limit=6700000 ;;
esac

if [[ ! -f "$binary" ]]; then
  echo "missing release binary: $binary" >&2
  exit 1
fi

size=$(wc -c < "$binary" | tr -d ' ')
echo "$binary: $size bytes (limit: <$limit bytes)"
if (( size >= limit )); then
  if (( baseline_mode )); then
    if [[ "$binary" != "target/release/orcacode" ]]; then
      echo "baseline allowance is restricted to target/release/orcacode" >&2
      exit 1
    fi
    baseline_file="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/baselines/orca-d89bf274.json"
    baseline_limit=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1], encoding="utf-8"))["linux_x86_64_gnu_release_binary_max_bytes"])' "$baseline_file")
    if (( size <= baseline_limit )); then
      echo "$binary: inherited Orca d89bf274 baseline, no growth (ceiling: $baseline_limit bytes)"
      exit 0
    fi
    echo "release binary exceeds the imported Orca d89bf274 baseline" >&2
    echo "$binary: $size bytes (baseline ceiling: $baseline_limit bytes)" >&2
    exit 1
  fi
  echo "release binary exceeds the total size limit" >&2
  exit 1
fi

if (( baseline_mode )) && [[ "$binary" != "target/release/orcacode" ]]; then
  echo "baseline allowance is restricted to target/release/orcacode" >&2
  exit 1
fi
