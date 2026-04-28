#!/usr/bin/env bash
# Maintainer: build all supported prebuilds into prebuilds/<triple>/.
#
# - Apple (macOS + iOS): requires macOS, Xcode toolchains, and Rust. Runs
#   tool/build_prebuilds.sh for every Apple triple.
# - Android: runs tool/build_android_prebuilds_docker.sh (Docker; Linux/amd64
#   NDK as documented in that script).
#
# Prebuilds are under prebuilds/ and are usually gitignored (see .gitignore);
# keep prebuilds/manifest.json in git.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

case "$(uname -s)" in
  Darwin) ;;
  *)
    echo "Apple triples require macOS with Xcode. On Linux, build Android only: ./tool/build_android_prebuilds_docker.sh" >&2
    exit 1
    ;;
esac

echo "==> Apple prebuilds (Rust)…"
"$ROOT/tool/build_prebuilds.sh" \
  aarch64-apple-darwin \
  x86_64-apple-darwin \
  aarch64-apple-ios \
  aarch64-apple-ios-sim \
  x86_64-apple-ios

echo "==> Android prebuilds (Docker)…"
"$ROOT/tool/build_android_prebuilds_docker.sh"

echo "==> Done. Update metadata if needed: ./tool/update_prebuild_metadata.sh (no --base-url for local hash-only manifest)"
