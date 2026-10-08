#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions only format diagnostics
# 0.8.28 Slice 30 qualification: every harness tolerance and safety number in
# one place. Sourced by lib-host.sh; each value can be overridden from the
# environment. These are test-harness settings, not product behaviour.
# Adapted from the pool study's study-config.sh (STUDY_* became QUAL_*).

# Host-quiet check: MemAvailable needed before a run starts, the 1-minute load
# average ceiling, and the GPU-idle sampling (seconds).
: "${QUAL_QUIET_MEM_KIB:=$((40 * 1024 * 1024))}"
: "${QUAL_QUIET_LOAD1_MAX:=2.0}"
: "${QUAL_GPU_IDLE_SAMPLES:=3}"
# 1: refuse a run while compilers, tests or a high load average are present
# (timing series, G9). 0: the functional gates ignore them and still require
# an idle GPU, no foreign GPU-node holder and enough memory.
: "${QUAL_QUIET_STRICT:=1}"
# Longest wait for a quiet host before a run is refused (seconds).
: "${QUAL_QUIET_WAIT_S:=1800}"

# Abort floor during a series (plan 4.2 G7): below this MemAvailable the series
# stops, so the shared desktop and session keep working memory.
: "${QUAL_MEM_FLOOR_KIB:=$((8 * 1024 * 1024))}"

# A NO_SWAP=1 series tolerates this rise over its series-start baseline.
: "${QUAL_SWAP_TOLERANCE_KIB:=1024}"

# Per-process time limit (seconds).
: "${QUAL_RUN_TIMEOUT_S:=900}"

export QUAL_QUIET_STRICT QUAL_QUIET_MEM_KIB QUAL_QUIET_LOAD1_MAX QUAL_GPU_IDLE_SAMPLES QUAL_QUIET_WAIT_S \
  QUAL_MEM_FLOOR_KIB QUAL_SWAP_TOLERANCE_KIB QUAL_RUN_TIMEOUT_S
