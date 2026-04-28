#!/usr/bin/env bash
# Maintainer: build all Android ABI prebuilds using Docker (Linux NDK inside image).
# Requires Docker. Copies artifacts to prebuilds/<triple>/ and prints shasum lines.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${VC_ZKP_ANDROID_IMAGE:-vc_zkp-android-prebuild}"

# Google's linux NDK zip only ships `toolchains/llvm/prebuilt/linux-x86_64`.
# Docker Desktop on Apple Silicon defaults to linux/arm64 guests, where those
# host binaries cannot run. Force amd64 so the NDK clang wrappers work.
PLATFORM="${VC_ZKP_DOCKER_PLATFORM:-linux/amd64}"

docker build --platform "$PLATFORM" \
  -f "$ROOT/tool/docker/android-prebuild.Dockerfile" \
  -t "$IMAGE" \
  "$ROOT/tool/docker"

docker run --rm --platform "$PLATFORM" \
  -e ANDROID_API_LEVEL="${ANDROID_API_LEVEL:-24}" \
  -v "$ROOT:/work" \
  -w /work \
  "$IMAGE" \
  bash /work/tool/docker/build_android_inner.sh

CRATE="$ROOT"
for triple in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android; do
  mkdir -p "$ROOT/prebuilds/$triple"
  cp -f "$CRATE/target/$triple/release/librust_eddsa_helper.so" "$ROOT/prebuilds/$triple/librust_eddsa_helper.so"
  if [[ -f "$CRATE/target/$triple/release/rust_eddsa_helper-${triple}.so.debug.zip" ]]; then
    cp -f "$CRATE/target/$triple/release/rust_eddsa_helper-${triple}.so.debug.zip" \
      "$ROOT/prebuilds/$triple/rust_eddsa_helper-${triple}.so.debug.zip"
  fi
  echo "==> installed $ROOT/prebuilds/$triple/librust_eddsa_helper.so"
  shasum -a 256 "$ROOT/prebuilds/$triple/librust_eddsa_helper.so"
  if [[ -f "$ROOT/prebuilds/$triple/rust_eddsa_helper-${triple}.so.debug.zip" ]]; then
    shasum -a 256 "$ROOT/prebuilds/$triple/rust_eddsa_helper-${triple}.so.debug.zip"
  fi
done

echo
echo "Update prebuilds/manifest.json with the sha256 lines printed above."
