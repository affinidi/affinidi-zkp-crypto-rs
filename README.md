# affinidi-zkp-crypto

`affinidi-zkp-crypto` is a standalone, high-performance Rust cryptographic library designed for zero-knowledge credential and proof workflows. It serves as the foundational cryptographic engine, providing core primitives for signing, verification, and hashing across multiple platforms.

The library is engineered for seamless integration into various environments, including native Rust services (desktop/server) and mobile runtimes (iOS/Android) via Foreign Function Interface (FFI).

> **SECURITY WARNING:**
> This library handles sensitive cryptographic operations. All usage requires developers to adhere to best practices regarding key management, storage, and secure memory handling. The library's security depends entirely on the secure integration and usage context of the application.

## Table of Contents

- [Core Concepts](#core-concepts)
- [Supported Crypto & Key Management](#supported-crypto--key-management)
- [Architecture & Platforms](#architecture--platforms)
- [Development & Prerequisites](#development--prerequisites)
- [Installation](#installation)
- [Usage](#usage)
- [Testing](#testing)
- [Support & Feedback](#support--feedback)
- [Contributing](#contributing)

## Core Concepts

- **EdDSA:** Elliptic Curve Digital Signature Algorithm. The library uses EdDSA signatures for verifying digital integrity.
- **BabyJubJub Curve:** The specific elliptic curve utilized for EdDSA signing. Using this curve ensures compatibility with standard ZKP tools like `circomlibjs`.
- **Poseidon Hash:** A permutation-based hash function essential for ZKP. It is used to deterministically hash structured inputs (commitments) into a fixed-size digest, which is then signed by EdDSA.
- **FFI (Foreign Function Interface):** This mechanism allows the compiled Rust code to be called directly from high-level languages (like Swift/Kotlin) and other runtimes, enabling multi-platform deployment.

## Supported Crypto & Key Management

The library provides three interlocking, critical capabilities:

1. **Hashing:** **Poseidon hashing** is supported over both **BN254 field elements** and raw **bit inputs**, ensuring consistent, circuit-friendly commitment generation.
2. **Signatures:** **EdDSA** signing and verification are implemented using the **BabyJubJub elliptic curve**. This combination guarantees the highest level of cryptographic robustness for digital identity claims.
3. **Multi-Platform Export:** The core logic is compiled to artifacts suitable for consumption across multiple runtime environments:
    *   Rust Services (`cdylib`)
    *   iOS (`staticlib` with exported C symbols)
    *   Android (`staticlib` with exported C symbols)

## Architecture & Platforms

The library is built using various formats to suit different deployment scenarios:

- **`cdylib`:** Used for linking in other Rust projects.
- **`staticlib`:** Ideal for linking into larger applications (e.g., Xcode project).
- **`rlib`:** Used for internal Rust module usage.

The artifacts are produced in the following locations based on the build profile:

- **macOS / iOS / iOS Simulator:** `librust_eddsa_helper.dylib`
- **Android / Linux:** `librust_eddsa_helper.so`

## Development & Prerequisites

This section details the tools required to build the library.

### Development Tools
Before building, ensure the following tools are installed:
- Rust toolchain via `rustup` (stable, Rust 2021 edition compatible).
- C/C++ build tooling:
    - **macOS:** Xcode Command Line Tools (`xcode-select --install`)
    - **Linux:** `build-essential` (or equivalent)

### Platform-Specific Targets
For iOS builds, the following targets must be added:

```bash
rustup target add aarch64-apple-ios x86_64-apple-ios
```

For Android builds, the NDK toolchain is required:
```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

### Build Commands

```bash
# Native host build
cargo build --release

# iOS (static lib with exported C symbols)
cargo build --profile ios --target aarch64-apple-ios
cargo build --profile ios --target x86_64-apple-ios

# Android (requires Android NDK/toolchain)
cargo build --release --target aarch64-linux-android
cargo build --release --target armv7-linux-androideabi
cargo build --release --target i686-linux-android
cargo build --release --target x86_64-linux-android
```

## Installation

*Since this library is a core, native component, installation typically involves linking the pre-compiled artifact rather than a standard package manager command.*

1. **Download Artifacts:** Obtain the necessary `librust_eddsa_helper.[so|a|dylib]` file corresponding to your target platform (Android, iOS, Linux).
2. **Integration:** Link the downloaded artifact into your target project (e.g., using an Xcode Framework or JNI library dependency).

## Usage

The usage methods vary significantly based on the target environment:

### Desktop (Rust CLI Example)

The crate includes a simple CLI binary (`eddsa_cli`) for signing, allowing verification of the entire flow:

**Sign with Precomputed Message Hash:**
```bash
cargo run --release --bin eddsa_cli -- \
  12345678901234567890 \
  000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f
```
*(Output: Signature JSON)*

**Sign with Raw Bits:**
```bash
cargo run --release --bin eddsa_cli -- \
  bits '[0,1,0,1,1,0]' \
  000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f
```

### iOS (Swift + FFI Example)

After building the static library and linking it into the Xcode project, call the exported C symbols:

```swift
import Foundation

@_silgen_name("eddsa_sign")
func eddsa_sign(_ inputJson: UnsafePointer<CChar>, _ outputJson: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>) -> Int32

// Example usage snippet...
```

### Android (Kotlin + JNI Example)

Use the generated JNI wrapper to interact with the underlying C function:

```kotlin
object NativeCrypto {
  init { System.loadLibrary("rust_eddsa_helper") }
  external fun eddsaSign(inputJson: String): String
}

val request = """{"operation":"sign","data":{"msgHash":"12345","privateKeyHex":"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"}}"""
val response = NativeCrypto.eddsaSign(request)
println(response)
```

## Testing

Testing the cryptographic primitives requires dedicated suites:

```bash
# Run unit tests
cargo test

# Check formatting consistency (recommended for CI)
cargo fmt --all -- --check
```

## Support & Feedback

If you encounter technical issues or have suggestions for improving the cryptographic primitives or platform bindings, please don't hesitate to contact us using [this link](https://share.hsforms.com/1i-4HKZRXSsmENzXtPdIG4g8oa2v).

### Reporting Technical Issues
For issues with the codebase, please open a detailed issue on GitHub. Please provide reproducible steps, the operating system, and the relevant environment configuration.

## Contributing

We welcome contributions!

Please review our [CONTRIBUTING](CONTRIBUTING.md) guidelines, which detail requirements for unit testing, architectural adherence, and pull request submission.
