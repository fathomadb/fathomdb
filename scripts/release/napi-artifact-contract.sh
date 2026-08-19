#!/usr/bin/env bash
# 0.8.23's one checked source for the containerized napi (linux-arm64-gnu)
# build inputs (Slice 80.1, R80-2). It is sourced only by the build-napi
# CI job and its local provisioning/build helpers.
#
# Mirrors the shape of scripts/release/cuda-artifact-contract.sh: image and
# toolchain identity live here so release.yml never inlines them, and the
# base image is digest-pinned so a moved `:latest` tag cannot silently
# change the build floor.
export NAPI_MANYLINUX_IMAGE='fathomdb-napi-manylinux:2_28-aarch64'
export NAPI_MANYLINUX_PLATFORM='linux/arm64'
export NAPI_MANYLINUX_BASE_IMAGE='quay.io/pypa/manylinux_2_28_aarch64@sha256:5bf033a700fd265da5cca83c0fa379810b8a4b516a2ec432f1a2b51dca4346cf'
export NAPI_MANYLINUX_DOCKERFILE='scripts/release/Dockerfile.napi-manylinux'
export NAPI_RUST_VERSION='1.95.0'
# Pins the toolchain the cargo/rustc rustup proxies resolve to, so they never
# attempt a channel-update sync (which needs to write under the image-owned,
# read-only RUSTUP_HOME) — the same discipline
# scripts/release/cuda-preflight.sh's CUDA_RUSTUP_TOOLCHAIN uses.
export NAPI_RUSTUP_TOOLCHAIN='1.95.0-aarch64-unknown-linux-gnu'
export NAPI_NODE_VERSION='25.9.0'
export NAPI_NODE_LINUX_ARM64_SHA256='bf007bf0dcc2fddd90888fde374a1ad33c1ab2ca2ad324c645dd7aed0f9f1460'
# rustup-init is a pinned bootstrap binary; rustup subsequently resolves the
# explicitly-versioned Rust toolchain above from static.rust-lang.org.
export NAPI_RUSTUP_INIT_URL='https://static.rust-lang.org/rustup/archive/1.29.0/aarch64-unknown-linux-gnu/rustup-init'
export NAPI_RUSTUP_INIT_SHA256='9732d6c5e2a098d3521fca8145d826ae0aaa067ef2385ead08e6feac88fa5792'
