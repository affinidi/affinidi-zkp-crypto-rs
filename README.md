# affinidi-zkp-crypto-rs

`affinidi-zkp-crypto-rs` is a standalone Rust crypto library for zero-knowledge credential and proof workflows. It is designed for use from both Rust services (desktop/server) and mobile runtimes (iOS/Android) via FFI.

The library provides three core capabilities:

- Poseidon hashing over BN254 field elements and bit inputs
- BabyJubJub EdDSA sign/verify primitives (circomlibjs-compatible flow)
- FFI exports for mobile/runtime integrations

These capabilities are intended to work together in a common flow:

1. Hash structured inputs with Poseidon.
2. Sign or verify the Poseidon digest with BabyJubJub EdDSA.
3. Call the same Rust implementation from mobile apps through exported C symbols.

The crate builds as:

- `cdylib`
- `staticlib`
- `rlib`

## Prerequisites

Install the following before building:

- Rust toolchain via `rustup` (stable, Rust 2021 edition compatible)
- C/C++ build tooling:
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Linux: `build-essential` (or equivalent)

For iOS builds:

- Xcode and iOS SDK
- Rust targets:

```bash
rustup target add aarch64-apple-ios x86_64-apple-ios
```

For Android builds:

- Android NDK (with LLVM toolchain)
- Rust targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

Recommended validation commands:

```bash
cargo --version
rustup target list --installed
```

## Build

```bash
# Native host build
cargo build --release

# iOS (static lib with exported C symbols)
cargo build --profile ios --target aarch64-apple-ios
cargo build --profile ios --target x86_64-apple-ios # Simulator

# Android (requires Android NDK/toolchain)
cargo build --release --target aarch64-linux-android
cargo build --release --target armv7-linux-androideabi
cargo build --release --target i686-linux-android
cargo build --release --target x86_64-linux-android
```

Artifacts are produced under `target/<triple>/<profile>/`:

- macOS: `librust_eddsa_helper.dylib`
- Linux: `librust_eddsa_helper.so`
- iOS: `librust_eddsa_helper.a` (from `--profile ios`)
- Android: `librust_eddsa_helper.so`

## Usage

### Desktop (Rust CLI example)

The crate includes a simple CLI binary (`eddsa_cli`) for signing.

```bash
# Build and run with a precomputed Poseidon message hash
cargo run --release --bin eddsa_cli -- \
  12345678901234567890 \
  000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f
```

It prints signature JSON:

```json
{"Ax":"...","Ay":"...","R8x":"...","R8y":"...","S":"..."}
```

You can also sign raw bits (CLI hashes with Poseidon internally):

```bash
cargo run --release --bin eddsa_cli -- \
  bits '[0,1,0,1,1,0]' \
  000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f
```

### iOS (Swift + FFI)

Build static library:

```bash
cargo build --profile ios --target aarch64-apple-ios
```

Link `librust_eddsa_helper.a` into your Xcode project, then call exported symbols:

```swift
import Foundation

@_silgen_name("eddsa_sign")
func eddsa_sign(_ inputJson: UnsafePointer<CChar>, _ outputJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32

@_silgen_name("poseidon_free_string")
func poseidon_free_string(_ ptr: UnsafeMutablePointer<CChar>?)

let request = #"{"operation":"sign","data":{"msgHash":"12345","privateKeyHex":"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"}}"#
var outPtr: UnsafeMutablePointer<CChar>?

let code = request.withCString { cstr in
  eddsa_sign(cstr, &outPtr)
}

if let outPtr {
  let response = String(cString: outPtr)
  poseidon_free_string(outPtr)
  print("code=\(code), response=\(response)")
}
```

### Android (Kotlin + JNI bridge)

Build shared library:

```bash
cargo build --release --target aarch64-linux-android
```

Example JNI bridge (`native-lib.cpp`) that wraps `eddsa_sign`:

```cpp
#include <jni.h>
#include <string>

extern "C" int eddsa_sign(const char* input_json, char** output_json);
extern "C" void poseidon_free_string(char* ptr);

extern "C" JNIEXPORT jstring JNICALL
Java_com_example_crypto_NativeCrypto_eddsaSign(JNIEnv* env, jobject, jstring inputJson) {
  const char* input = env->GetStringUTFChars(inputJson, nullptr);
  char* output = nullptr;
  int code = eddsa_sign(input, &output);
  env->ReleaseStringUTFChars(inputJson, input);

  std::string result = output ? output : "{\"success\":false,\"error\":\"null output\"}";
  if (output) {
    poseidon_free_string(output);
  }

  return env->NewStringUTF(result.c_str());
}
```

Example Kotlin usage:

```kotlin
object NativeCrypto {
  init { System.loadLibrary("rust_eddsa_helper") }
  external fun eddsaSign(inputJson: String): String
}

val request = """{"operation":"sign","data":{"msgHash":"12345","privateKeyHex":"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"}}"""
val response = NativeCrypto.eddsaSign(request)
println(response)
```

## Test

```bash
# Run unit tests
cargo test

# Format check (optional but recommended in CI/local pre-commit)
cargo fmt --all -- --check
```

Current test coverage includes:

- sign->verify happy-path checks
- verification failures for wrong public key and wrong message hash
- malformed verification payload handling
  - core EdDSA API error propagation
  - FFI wrapper error JSON contract (`success: false`)

## CI

Workflows are under `.github/workflows/`:

- `rust-tests.yaml`: runs crate test suite on PRs/pushes
- `checks.yaml` and `release.yaml`: reusable pipeline integration
- `build-prebuilds.yml`: native prebuild pipeline and release assets

