#!/usr/bin/env bash
# 0.8.23's one checked source for the declared glibc floor per artifact
# family (R80-2, AC80-1, AC80-9, AC80-26). scripts/check-glibc-floor.sh
# callers and docs/compatibility/index.md's doc-truth fixture both read this
# file so the floor cannot drift between the gate and the published claim.
#
# 0.8.23 Slice 80.6 (D-80.6-5): the floor is now declared PER ARTIFACT FAMILY.
# The Jetson/Tegra CUDA artifact is built host-natively against JetPack's own
# glibc, so its floor is genuinely higher than the containerized families'.
# That is declared here and asserted by the same gate — never exempted from it.
#
# scripts/check-glibc-floor.sh itself needs no family notion: it already takes
# an arbitrary `--floor X.Y`. Only this contract and the call sites choosing
# WHICH floor to pass carry the per-family logic.

# GLIBC_FLOOR_FAMILIES is the checked list of families. The doc-truth gate
# iterates it, so adding a family here without stating its floor in
# docs/compatibility/index.md fails closed rather than going unchecked.
export GLIBC_FLOOR_FAMILIES='manylinux tegra'

# manylinux: every artifact built inside the manylinux_2_28 image — the Python
# wheels (build-python) and, as of Slice 80.1, the containerized
# linux-arm64-gnu napi binding (build-napi).
export GLIBC_FLOOR_MANYLINUX='2.28'

# tegra: the host-native Jetson Orin CUDA Python wheel
# (scripts/release/build-python-cuda-tegra.sh, D-80.6-2 — the wheel only).
# Measured on the Jetson Orin AGX: Ubuntu GLIBC 2.35-0ubuntu3.14.
export GLIBC_FLOOR_TEGRA='2.35'

# The bare GLIBC_FLOOR keeps its pre-80.6 meaning and its pre-80.6 value for
# every existing call site (release.yml's napi gate, the two published-artifact
# smokes), so those lanes are unchanged in strength (AC80-1, AC80-2).
export GLIBC_FLOOR="$GLIBC_FLOOR_MANYLINUX"

# Resolve a family name to its declared floor, failing closed on an unknown
# family rather than returning an empty --floor argument.
glibc_floor_for_family() {
  case "${1:-}" in
    manylinux) printf '%s\n' "$GLIBC_FLOOR_MANYLINUX" ;;
    tegra) printf '%s\n' "$GLIBC_FLOOR_TEGRA" ;;
    *)
      printf 'glibc-floor-contract: unknown artifact family: %s\n' "${1:-}" >&2
      return 1
      ;;
  esac
}
