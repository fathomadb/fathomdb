#!/usr/bin/env bash
# 0.8.23's one checked source for the declared glibc floor per artifact
# family (R80-2, AC80-1, AC80-9). scripts/check-glibc-floor.sh callers and
# docs/compatibility/index.md's doc-truth fixture both read this file so the
# floor cannot drift between the gate and the published claim.
#
# GLIBC_FLOOR covers every artifact built inside the manylinux_2_28 image:
# the Python wheels (build-python) and, as of Slice 80.1, the containerized
# linux-arm64-gnu napi binding (build-napi). It is intentionally NOT the
# floor for the Jetson/Tegra CUDA artifact (Slice 80.6), which is built
# host-bound against JetPack's own glibc and is asserted separately.
export GLIBC_FLOOR='2.28'
