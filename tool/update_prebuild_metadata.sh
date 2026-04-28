#!/usr/bin/env bash
# Recompute prebuild manifest (local files) + source fingerprint.
#
# Usage:
#   ./tool/update_prebuild_metadata.sh
#
# What it does:
#   1) Scans prebuilds/<triple>/ for librust_eddsa_helper.{dylib,so}
#   2) Rewrites prebuilds/manifest.json with fresh sha256 values from local files
#   3) Regenerates prebuilds/source_fingerprint.txt
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$ROOT/prebuilds/manifest.json"
FINGERPRINT="$ROOT/prebuilds/source_fingerprint.txt"

if [[ ! -d "$ROOT/prebuilds" ]]; then
  echo "Missing directory: $ROOT/prebuilds" >&2
  exit 1
fi

"$ROOT/tool/generate_manifest_from_assets.sh" \
  --assets-root "$ROOT/prebuilds" \
  --output "$MANIFEST"

"$ROOT/tool/compute_rust_source_fingerprint.sh" > "$FINGERPRINT"
echo "Updated $FINGERPRINT"

