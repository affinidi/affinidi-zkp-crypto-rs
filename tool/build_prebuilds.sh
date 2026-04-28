#!/usr/bin/env bash
# Maintainer-only: compile this crate for one or more Rust triples
# and install artifacts under prebuilds/<triple>/.
#
# Also produces symbol bundles for crash log symbolication (Apple: .dSYM zip;
# Android: use tool/build_android_prebuilds_docker.sh and llvm-objcopy there).
#
# Apple (macOS / iOS): sets SDK / deployment targets automatically.
# Android: requires ANDROID_NDK_ROOT (or ANDROID_HOME/ndk/*) on the host, or use
#           tool/build_android_prebuilds_docker.sh instead.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CRATE="$ROOT"

lib_name_for_target() {
  case "$1" in
    *linux-android*) echo "librust_eddsa_helper.so" ;;
    *) echo "librust_eddsa_helper.dylib" ;;
  esac
}

# Cargo puts Apple *iOS* cdylib under target/<triple>/ios/ (not .../release/).
resolve_built_library() {
  local triple="$1"
  local lib="$2"
  local p
  for p in "$CRATE/target/$triple/release/$lib" "$CRATE/target/$triple/ios/$lib"; do
    if [[ -f "$p" ]]; then
      echo "$p"
      return 0
    fi
  done
  p="$(find "$CRATE/target/$triple" -name "$lib" -type f 2>/dev/null | grep -v /deps/ | head -1 || true)"
  if [[ -n "$p" && -f "$p" ]]; then
    echo "$p"
    return 0
  fi
  echo "Could not find built $lib under $CRATE/target/$triple" >&2
  return 1
}

# Apple: zip a dSYM bundle for crash symbolication. Prefer rustc-emitted *.dSYM;
# if missing (common with thin LTO), run `dsymutil` on the built dylib.
package_apple_dsym() {
  local triple="$1"
  local built="$2"
  local out_zip
  out_zip="$ROOT/prebuilds/$triple/rust_eddsa_helper-${triple}.dSYM.zip"
  local dsym
  dsym="$(find "$CRATE/target/$triple" -name 'librust_eddsa_helper*.dSYM' -type d 2>/dev/null | head -1 || true)"
  local work=""
  if [[ -z "$dsym" || ! -d "$dsym" ]]; then
    work="$(mktemp -d "${TMPDIR:-/tmp}/vc_zkp_dsym.XXXXXX")"
    if ! xcrun dsymutil "$built" -o "$work/librust_eddsa_helper.dylib.dSYM"; then
      echo "ERROR: dSYM not found for $triple and dsymutil failed." >&2
      rm -rf "$work"
      return 1
    fi
    dsym="$work/librust_eddsa_helper.dylib.dSYM"
  fi
  rm -f "$out_zip"
  ( cd "$(dirname "$dsym")" && zip -r -q "$out_zip" "$(basename "$dsym")" )
  [[ -n "$work" ]] && rm -rf "$work"
  echo "==> dSYM $out_zip"
  shasum -a 256 "$out_zip"
}

apply_apple_env() {
  local triple="$1"
  unset CARGO_TARGET_DIR RUSTFLAGS SDKROOT IPHONEOS_DEPLOYMENT_TARGET MACOSX_DEPLOYMENT_TARGET
  case "$triple" in
    aarch64-apple-darwin | x86_64-apple-darwin)
      export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-12.0}"
      ;;
    aarch64-apple-ios)
      export SDKROOT="$(xcrun --sdk iphoneos --show-sdk-path)"
      export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-13.0}"
      export RUSTFLAGS="-C link-arg=-isysroot -C link-arg=$SDKROOT"
      ;;
    aarch64-apple-ios-sim | x86_64-apple-ios)
      export SDKROOT="$(xcrun --sdk iphonesimulator --show-sdk-path)"
      export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-13.0}"
      export RUSTFLAGS="-C link-arg=-isysroot -C link-arg=$SDKROOT"
      ;;
    *)
      return 1
      ;;
  esac
  return 0
}

ndk_host_tag() {
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) echo "darwin-arm64" ;;
    Darwin-x86_64) echo "darwin-x86_64" ;;
    Linux-x86_64) echo "linux-x86_64" ;;
    Linux-aarch64) echo "linux-aarch64" ;;
    *) echo "" ;;
  esac
}

resolve_ndk_root() {
  if [[ -n "${ANDROID_NDK_ROOT:-}" && -d "${ANDROID_NDK_ROOT}" ]]; then
    echo "$ANDROID_NDK_ROOT"
    return
  fi
  if [[ -n "${ANDROID_NDK_HOME:-}" && -d "${ANDROID_NDK_HOME}" ]]; then
    echo "$ANDROID_NDK_HOME"
    return
  fi
  if [[ -n "${ANDROID_HOME:-}" ]]; then
    local best
    best="$(ls -d "$ANDROID_HOME/ndk/"* 2>/dev/null | sort -V | tail -1 || true)"
    if [[ -n "$best" && -d "$best" ]]; then
      echo "$best"
      return
    fi
  fi
  echo ""
}

apply_android_env() {
  local triple="$1"
  local ndk api bin host
  ndk="$(resolve_ndk_root)"
  if [[ -z "$ndk" ]]; then
    echo "Android triple $triple: set ANDROID_NDK_ROOT or ANDROID_HOME with ndk/, or run tool/build_android_prebuilds_docker.sh" >&2
    return 1
  fi
  host="$(ndk_host_tag)"
  if [[ -z "$host" ]]; then
    echo "Unsupported host for Android NDK: $(uname -s)-$(uname -m)" >&2
    return 1
  fi
  bin="$ndk/toolchains/llvm/prebuilt/$host/bin"
  if [[ ! -d "$bin" ]]; then
    echo "NDK llvm bin not found: $bin" >&2
    return 1
  fi
  api="${ANDROID_API_LEVEL:-24}"

  unset CC_aarch64_linux_android CC_armv7_linux_androideabi CC_x86_64_linux_android CC_i686_linux_android
  unset AR_aarch64_linux_android AR_armv7_linux_androideabi AR_x86_64_linux_android AR_i686_linux_android
  unset CFLAGS_aarch64_linux_android CFLAGS_armv7_linux_androideabi CFLAGS_x86_64_linux_android CFLAGS_i686_linux_android
  unset CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER CARGO_TARGET_I686_LINUX_ANDROID_LINKER

  case "$triple" in
    aarch64-linux-android)
      export CC_aarch64_linux_android="$bin/aarch64-linux-android${api}-clang"
      export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
      export AR_aarch64_linux_android="$bin/llvm-ar"
      export CFLAGS_aarch64_linux_android="-D__ANDROID_API__=$api"
      ;;
    armv7-linux-androideabi)
      export CC_armv7_linux_androideabi="$bin/armv7a-linux-androideabi${api}-clang"
      export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$CC_armv7_linux_androideabi"
      export AR_armv7_linux_androideabi="$bin/llvm-ar"
      export CFLAGS_armv7_linux_androideabi="-D__ANDROID_API__=$api"
      ;;
    x86_64-linux-android)
      export CC_x86_64_linux_android="$bin/x86_64-linux-android${api}-clang"
      export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$CC_x86_64_linux_android"
      export AR_x86_64_linux_android="$bin/llvm-ar"
      export CFLAGS_x86_64_linux_android="-D__ANDROID_API__=$api"
      ;;
    i686-linux-android)
      export CC_i686_linux_android="$bin/i686-linux-android${api}-clang"
      export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="$CC_i686_linux_android"
      export AR_i686_linux_android="$bin/llvm-ar"
      export CFLAGS_i686_linux_android="-D__ANDROID_API__=$api"
      ;;
    *)
      return 1
      ;;
  esac
  return 0
}

export_for_triple() {
  local triple="$1"
  unset CARGO_TARGET_DIR RUSTFLAGS SDKROOT IPHONEOS_DEPLOYMENT_TARGET MACOSX_DEPLOYMENT_TARGET
  unset CC_aarch64_linux_android CC_armv7_linux_androideabi CC_x86_64_linux_android CC_i686_linux_android
  unset AR_aarch64_linux_android AR_armv7_linux_androideabi AR_x86_64_linux_android AR_i686_linux_android
  unset CFLAGS_aarch64_linux_android CFLAGS_armv7_linux_androideabi CFLAGS_x86_64_linux_android CFLAGS_i686_linux_android
  unset CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER CARGO_TARGET_I686_LINUX_ANDROID_LINKER

  case "$triple" in
    *linux-android*)
      apply_android_env "$triple"
      ;;
    *)
      apply_apple_env "$triple" || true
      ;;
  esac
}

build_one() {
  local triple="$1"
  local lib
  lib="$(lib_name_for_target "$triple")"
  export_for_triple "$triple"
  # Line tables + do not strip so Apple dSYM and Android objcopy can emit separate debug packages.
  export CARGO_PROFILE_RELEASE_DEBUG="${CARGO_PROFILE_RELEASE_DEBUG:-line-tables-only}"
  export CARGO_PROFILE_RELEASE_STRIP="${CARGO_PROFILE_RELEASE_STRIP:-none}"
  # Fat LTO can make dSYMs unhelpful; thin LTO is a good default for prebuilds (still optimized).
  case "$triple" in
    *apple*) export CARGO_PROFILE_RELEASE_LTO="${CARGO_PROFILE_RELEASE_LTO:-thin}" ;;
  esac
  echo "==> cargo build --release --target $triple"
  (cd "$CRATE" && cargo build --release --target "$triple")
  mkdir -p "$ROOT/prebuilds/$triple"
  local built
  built="$(resolve_built_library "$triple" "$lib")" || exit 1
  cp -f "$built" "$ROOT/prebuilds/$triple/$lib"
  echo "==> installed $ROOT/prebuilds/$triple/$lib"
  shasum -a 256 "$ROOT/prebuilds/$triple/$lib"
  case "$triple" in
    *apple*)
      package_apple_dsym "$triple" "$built" || exit 1
      ;;
  esac
}

if [[ "${#@}" -eq 0 ]]; then
  echo "Usage: $0 <rust-triple> [more-triples...]"
  echo "Examples:"
  echo "  $0 aarch64-apple-darwin"
  echo "  $0 aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios"
  echo "  ANDROID_NDK_ROOT=... $0 aarch64-linux-android armv7-linux-androideabi x86_64-linux-android i686-linux-android"
  exit 1
fi

for t in "$@"; do
  build_one "$t"
done

echo
echo "Update prebuilds/manifest.json: set \"sha256\" for each triple to the shasum above."
