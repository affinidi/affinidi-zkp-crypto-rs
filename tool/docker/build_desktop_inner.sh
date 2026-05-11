#!/usr/bin/env bash
# Runs inside desktop-prebuild Docker image. CWD must be crate root.
set -euo pipefail

export CARGO_PROFILE_RELEASE_DEBUG="${CARGO_PROFILE_RELEASE_DEBUG:-line-tables-only}"
export CARGO_PROFILE_RELEASE_STRIP="${CARGO_PROFILE_RELEASE_STRIP:-none}"
export CARGO_PROFILE_RELEASE_LTO="${CARGO_PROFILE_RELEASE_LTO:-thin}"

# ── Linux aarch64 cross-compile ─────────────────────────────────────────────
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

# ── Windows x64 cross-compile via MinGW ────────────────────────────────────
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc

for triple in \
  x86_64-unknown-linux-gnu \
  aarch64-unknown-linux-gnu \
  x86_64-pc-windows-gnu; do
  echo "==> cargo build --release --target $triple"
  cargo build --release --target "$triple"
done
