#!/usr/bin/env bash
# Build iOS static lib (profile ios = no LTO, symbols visible) and verify C symbols.
set -e
cd "$(dirname "$0")"
cargo clean
cargo build --profile ios --target aarch64-apple-ios

resolve_ios_static_lib() {
  local candidate
  for candidate in \
    target/aarch64-apple-ios/ios/librust_eddsa_helper.a \
    target/aarch64-apple-ios/release/librust_eddsa_helper.a; do
    if [[ -f "$candidate" ]]; then
      echo "$candidate"
      return 0
    fi
  done
  echo "Could not find iOS static lib in expected locations." >&2
  return 1
}

LIB="$(resolve_ios_static_lib)"
echo "--- Artifact: $LIB ---"
ls -la "$LIB"
echo "--- Symbols (eddsa) ---"
xcrun nm "$LIB" 2>/dev/null | grep -E 'eddsa' || { echo "eddsa_sign NOT IN RUST"; exit 1; }
echo "OK: eddsa symbols present in archive."
