#!/usr/bin/env bash
# Generate prebuilds/manifest.json from local asset files.
#
# Example:
#   ./tool/generate_manifest_from_assets.sh \
#     --assets-root dist/prebuilds \
#     --output prebuilds/manifest.json \
#     --base-url "https://github.com/org/repo/releases/download/prebuilds-<fp>"
set -euo pipefail

ASSETS_ROOT=""
OUTPUT=""
BASE_URL=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --assets-root)
      ASSETS_ROOT="$2"
      shift 2
      ;;
    --output)
      OUTPUT="$2"
      shift 2
      ;;
    --base-url)
      BASE_URL="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$ASSETS_ROOT" || -z "$OUTPUT" ]]; then
  echo "Usage: $0 --assets-root <dir> --output <manifest.json> [--base-url <url>]" >&2
  exit 1
fi

python3 - "$ASSETS_ROOT" "$OUTPUT" "$BASE_URL" <<'PY'
import hashlib
import json
import os
import sys

assets_root, output, base_url = sys.argv[1], sys.argv[2], sys.argv[3]
if not os.path.isdir(assets_root):
    raise SystemExit(f"assets root does not exist: {assets_root}")

entries = {}
for triple in sorted(os.listdir(assets_root)):
    triple_dir = os.path.join(assets_root, triple)
    if not os.path.isdir(triple_dir):
        continue

    lib_file = None
    for name in ("librust_eddsa_helper.dylib", "librust_eddsa_helper.so", "rust_eddsa_helper.dll"):
        candidate = os.path.join(triple_dir, name)
        if os.path.isfile(candidate):
            lib_file = candidate
            break
    if lib_file is None:
        continue

    with open(lib_file, "rb") as f:
        sha = hashlib.sha256(f.read()).hexdigest()

    entry = {"sha256": sha}
    if base_url:
        if lib_file.endswith(".dylib"):
            ext = "dylib"
        elif lib_file.endswith(".dll"):
            ext = "dll"
        else:
            ext = "so"
        asset_name = f"rust_eddsa_helper-{triple}.{ext}"
        entry["url"] = f"{base_url.rstrip('/')}/{asset_name}"

    dsym_zip = os.path.join(triple_dir, f"rust_eddsa_helper-{triple}.dSYM.zip")
    dbg_zip = os.path.join(triple_dir, f"rust_eddsa_helper-{triple}.so.debug.zip")
    if os.path.isfile(dsym_zip):
        with open(dsym_zip, "rb") as f:
            ssha = hashlib.sha256(f.read()).hexdigest()
        sym = {"kind": "dsym", "sha256": ssha}
        if base_url:
            sym["url"] = f"{base_url.rstrip('/')}/rust_eddsa_helper-{triple}.dSYM.zip"
        entry["symbols"] = sym
    elif os.path.isfile(dbg_zip):
        with open(dbg_zip, "rb") as f:
            ssha = hashlib.sha256(f.read()).hexdigest()
        sym = {"kind": "elf_debug", "sha256": ssha}
        if base_url:
            sym["url"] = f"{base_url.rstrip('/')}/rust_eddsa_helper-{triple}.so.debug.zip"
        entry["symbols"] = sym
    entries[triple] = entry

if not entries:
    raise SystemExit("No prebuild assets found; refusing to write empty manifest")

os.makedirs(os.path.dirname(output), exist_ok=True)
with open(output, "w", encoding="utf-8") as f:
    json.dump(entries, f, indent=2, sort_keys=True)
    f.write("\n")

print(f"Wrote {output} with {len(entries)} entries")
PY
