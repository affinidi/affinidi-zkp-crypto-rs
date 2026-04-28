#!/usr/bin/env bash
# Runs inside android-prebuild Docker image. CWD must be crate root.
set -euo pipefail

API="${ANDROID_API_LEVEL:-24}"
NDK="${ANDROID_NDK_ROOT:?ANDROID_NDK_ROOT not set}"

# Pick the NDK host prebuilt that matches this container (linux-aarch64 vs linux-x86_64).
# `find ... | head -1` is unsafe: on ARM64 Linux it may pick x86_64 and fail under Rosetta.
ndk_prebuilt_for_host() {
  local host
  host="$(uname -m)"
  case "$host" in
    aarch64 | arm64)
      if [[ -d "$NDK/toolchains/llvm/prebuilt/linux-aarch64" ]]; then
        echo "$NDK/toolchains/llvm/prebuilt/linux-aarch64"
        return
      fi
      ;;
    x86_64 | amd64)
      if [[ -d "$NDK/toolchains/llvm/prebuilt/linux-x86_64" ]]; then
        echo "$NDK/toolchains/llvm/prebuilt/linux-x86_64"
        return
      fi
      ;;
  esac
  local d
  d="$(find "$NDK/toolchains/llvm/prebuilt" -maxdepth 1 -mindepth 1 -type d | head -1 || true)"
  if [[ -n "$d" ]]; then
    echo "$d"
    return
  fi
  echo "No llvm prebuilt under $NDK/toolchains/llvm/prebuilt for host $host" >&2
  return 1
}

PREBUILT="$(ndk_prebuilt_for_host)"
BIN="$PREBUILT/bin"

# Match tool/build_prebuilds.sh: keep debug in .so for llvm-objcopy, then we ship a separate debug zip.
export CARGO_PROFILE_RELEASE_DEBUG="${CARGO_PROFILE_RELEASE_DEBUG:-line-tables-only}"
export CARGO_PROFILE_RELEASE_STRIP="${CARGO_PROFILE_RELEASE_STRIP:-none}"
# Thin LTO: fat LTO can interfere with objcopy --only-keep-debug; keep aligned with Apple prebuild defaults.
export CARGO_PROFILE_RELEASE_LTO="${CARGO_PROFILE_RELEASE_LTO:-thin}"

export CC_aarch64_linux_android="$BIN/aarch64-linux-android${API}-clang"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
export AR_aarch64_linux_android="$BIN/llvm-ar"
export CFLAGS_aarch64_linux_android="-D__ANDROID_API__=$API"

export CC_armv7_linux_androideabi="$BIN/armv7a-linux-androideabi${API}-clang"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$CC_armv7_linux_androideabi"
export AR_armv7_linux_androideabi="$BIN/llvm-ar"
export CFLAGS_armv7_linux_androideabi="-D__ANDROID_API__=$API"

export CC_x86_64_linux_android="$BIN/x86_64-linux-android${API}-clang"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$CC_x86_64_linux_android"
export AR_x86_64_linux_android="$BIN/llvm-ar"
export CFLAGS_x86_64_linux_android="-D__ANDROID_API__=$API"

export CC_i686_linux_android="$BIN/i686-linux-android${API}-clang"
export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="$CC_i686_linux_android"
export AR_i686_linux_android="$BIN/llvm-ar"
export CFLAGS_i686_linux_android="-D__ANDROID_API__=$API"

for triple in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android; do
  echo "==> cargo build --release --target $triple"
  cargo build --release --target "$triple"
done

echo "==> Android split debug (llvm-objcopy) + zip (upload next to .so in CI)"
for triple in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android; do
  rel="target/${triple}/release"
  ( cd "$rel" && \
    "$BIN/llvm-objcopy" --only-keep-debug "librust_eddsa_helper.so" "librust_eddsa_helper.so.debug" && \
    zip -q "rust_eddsa_helper-${triple}.so.debug.zip" "librust_eddsa_helper.so.debug" && \
    rm -f "librust_eddsa_helper.so.debug" )
  ls -la "target/${triple}/release/rust_eddsa_helper-${triple}.so.debug.zip"
  shasum -a 256 "target/${triple}/release/rust_eddsa_helper-${triple}.so.debug.zip"
done

echo "==> Android .so artifacts:"
for triple in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android; do
  ls -la "target/$triple/release/librust_eddsa_helper.so"
  shasum -a 256 "target/$triple/release/librust_eddsa_helper.so"
done
