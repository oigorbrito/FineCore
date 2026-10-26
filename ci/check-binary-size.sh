#!/usr/bin/env bash
set -euo pipefail

binary="${1:-target/release/finecore}"
# Static musl builds carry their own libc and allocator, about 2 MB more
# than the macOS binary. The inherited 6.7/9 MB limits are upstream
# release constraints; FineCore has not yet established a release baseline.
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
  echo "release binary exceeds the total size limit" >&2
  exit 1
fi
