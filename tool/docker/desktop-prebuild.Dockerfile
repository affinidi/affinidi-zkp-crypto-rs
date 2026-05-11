# Maintainer-only: Rust cross-compile image for Linux and Windows desktop prebuilds.
# Build:  docker build -f tool/docker/desktop-prebuild.Dockerfile -t vc_zkp-desktop-prebuild tool/docker
# Run:    see tool/build_desktop_prebuilds_docker.sh
FROM rust:1.85-bookworm

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    zip \
    gcc-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    gcc-mingw-w64-x86-64 \
  && rm -rf /var/lib/apt/lists/*

RUN rustup target add \
  x86_64-unknown-linux-gnu \
  aarch64-unknown-linux-gnu \
  x86_64-pc-windows-gnu
