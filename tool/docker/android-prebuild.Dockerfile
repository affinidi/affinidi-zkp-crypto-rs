# Maintainer-only: Rust + Android NDK for producing prebuild/*.so slices.
# Build:  docker build -f tool/docker/android-prebuild.Dockerfile -t vc_zkp-android-prebuild tool/docker
# Run:     see tool/build_android_prebuilds_docker.sh
FROM rust:1.85-bookworm

ARG ANDROID_NDK_VERSION=r26d
RUN apt-get update \
  && apt-get install -y --no-install-recommends zip unzip ca-certificates \
  && curl -fsSL -o /tmp/ndk.zip \
    "https://dl.google.com/android/repository/android-ndk-${ANDROID_NDK_VERSION}-linux.zip" \
  && unzip -q /tmp/ndk.zip -d /opt \
  && rm /tmp/ndk.zip \
  && mv /opt/android-ndk-* /opt/android-ndk

ENV ANDROID_NDK_ROOT=/opt/android-ndk

RUN rustup target add \
  aarch64-linux-android \
  armv7-linux-androideabi \
  x86_64-linux-android \
  i686-linux-android
