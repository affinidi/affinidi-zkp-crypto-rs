# Contributing

When contributing to this repository, please first discuss the change you wish to make by creating a new [GitHub issue](https://github.com/affinidi/affinidi-zkp-crypto-rs/issues/new).

## Development Requirements

### Installation

Install the required tooling on your machine:

- Rust toolchain (stable) via `rustup`
- Cargo (bundled with Rust)
- C/C++ build tooling
  - macOS: Xcode Command Line Tools (`xcode-select --install`)
  - Linux: `build-essential` (or equivalent)

Verify your Rust installation:

```bash
rustc --version
cargo --version
```

If you are contributing to mobile or desktop build support, install cross-compilation targets.

iOS targets:

```bash
rustup target add aarch64-apple-ios x86_64-apple-ios
```

Android targets:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

Linux desktop and Windows targets (via Docker — no local toolchain required):

```bash
bash tool/build_desktop_prebuilds_docker.sh
```

### Build

Run at least one local build before opening a PR:

```bash
# Native host build
cargo build --release

# iOS static library build profile
cargo build --profile ios --target aarch64-apple-ios

# Android shared library build
cargo build --release --target aarch64-linux-android
```

### Testing

This crate contains unit tests for Poseidon hashing and BabyJubJub EdDSA sign/verify flows.

Run tests locally:

```bash
# Full test suite
cargo test

# Optional: run only library tests
cargo test --lib
```

Recommended pre-commit checks:

```bash
# Format (writes changes)
cargo fmt --all

# Format check (CI-friendly)
cargo fmt --all -- --check

# Lint
cargo clippy --all-targets --all-features
```


### Code quality expectations

1. Ensure the pipeline checks are finished successfully.
2. Ensure the pull request doesn't contain redundant comments, console.log, etc.
3. Ensure your code is covered with automated tests (unit tests are required; add integration tests where applicable).
4. Avoid adding comments to explain what code does, code should be self-explanatory and clean.
5. Avoid using variable names like `i` or abbreviations - names should be simple and unambiguous.

### Pull request checklist

Before requesting review:

1. Rebase or merge from the latest `main`.
2. Confirm `cargo test` passes locally.
3. Confirm formatting and lint checks are clean.
4. Update docs (`README.md`, this file, or inline docs) when behavior or APIs change.
5. Add or update tests for new behavior and bug fixes.

## Code of Conduct

### Our Pledge

In the interest of fostering an open and welcoming environment, we as
contributors and maintainers pledge to make participation in our project and
our community a harassment-free experience for everyone, regardless of age, body
size, disability, ethnicity, gender identity and expression, level of experience,
nationality, personal appearance, race, religion, or sexual identity and
orientation.

### Our Standards

Examples of behavior that contributes to creating a positive environment
include:

- Using welcoming and inclusive language
- Being respectful of differing viewpoints and experiences
- Gracefully accepting constructive criticism
- Focusing on what is best for the community
- Showing empathy towards other community members
- Avoiding obvious comments about things like code styling and indentation.
  ** If you see yourself wanting to do that more than once - open an issue with a repo to update the ESLint/Prettier rules to address this concern once and for all. **Code reviews should be about logic, not indenting or adding more newlines\*\*

Examples of unacceptable behavior by participants include:

- The use of sexualized language or imagery and unwelcome sexual attention or
  advances
- Trolling, insulting/derogatory comments, and personal or political attacks
- Public or private harassment
- Publishing others' private information, such as a physical or electronic
  address, without explicit permission
- Other conduct which could reasonably be considered inappropriate in a
  professional setting