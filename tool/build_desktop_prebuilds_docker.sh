#!/usr/bin/env bash
# Maintainer: build Linux (x64, arm64) and Windows (x64) prebuilds using Docker.
# Requires Docker. Copies artifacts to prebuilds/<triple>/ and prints shasum lines.
#
# Linux targets:   x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu
# Windows target:  x86_64-pc-windows-gnu
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${VC_ZKP_DESKTOP_IMAGE:-vc_zkp-desktop-prebuild}"

# Build the Docker image (linux/amd64 for reproducibility with cross-compile toolchain).
PLATFORM="${VC_ZKP_DOCKER_PLATFORM:-linux/amd64}"

docker build --platform "$PLATFORM" \
  -f "$ROOT/tool/docker/desktop-prebuild.Dockerfile" \
  -t "$IMAGE" \
  "$ROOT/tool/docker"

docker run --rm --platform "$PLATFORM" \
  -v "$ROOT:/work" \
  -w /work \
  "$IMAGE" \
  bash /work/tool/docker/build_desktop_inner.sh

CRATE="$ROOT"

# ── Linux .so ────────────────────────────────────────────────────────────────
for triple in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
  mkdir -p "$ROOT/prebuilds/$triple"
  cp -f "$CRATE/target/$triple/release/librust_eddsa_helper.so" \
    "$ROOT/prebuilds/$triple/librust_eddsa_helper.so"
  echo "==> installed $ROOT/prebuilds/$triple/librust_eddsa_helper.so"
  shasum -a 256 "$ROOT/prebuilds/$triple/librust_eddsa_helper.so"
done

# ── Windows .dll ─────────────────────────────────────────────────────────────
for triple in x86_64-pc-windows-gnu; do
  mkdir -p "$ROOT/prebuilds/$triple"
  cp -f "$CRATE/target/$triple/release/rust_eddsa_helper.dll" \
    "$ROOT/prebuilds/$triple/rust_eddsa_helper.dll"
  echo "==> installed $ROOT/prebuilds/$triple/rust_eddsa_helper.dll"
  shasum -a 256 "$ROOT/prebuilds/$triple/rust_eddsa_helper.dll"
done

echo
echo "Update prebuilds/manifest.json with the sha256 lines printed above."
