#!/usr/bin/env bash
# 0.8.28 pool study: every harness tolerance and safety number in one place
# (owner ruling 17). Sourced by lib-host.sh; each value can be overridden from
# the environment. These are test-harness settings, not product behaviour.

# Host-quiet check (protocol 1.1): MemAvailable needed before a run starts,
# the 1-minute load average ceiling, and the GPU-idle sampling (seconds).
# 40 GiB leaves room for the largest probe (8 GiB cap x 2 + 16 GiB margin)
# on this 64 GB host; load 2.0 on 12 cores keeps builds and CI off the GPU.
: "${STUDY_QUIET_MEM_KIB:=$((40 * 1024 * 1024))}"
: "${STUDY_QUIET_LOAD1_MAX:=2.0}"
: "${STUDY_GPU_IDLE_SAMPLES:=3}"
# Longest wait for a quiet host before a run is refused (seconds).
: "${STUDY_QUIET_WAIT_S:=7200}"

# Abort floor during a series (protocol 1.2/1.3): below this MemAvailable the
# series stops, so the shared desktop and session keep working memory.
: "${STUDY_MEM_FLOOR_KIB:=$((8 * 1024 * 1024))}"

# Swap (ruling 8, ruling 17): a NO_SWAP=1 series tolerates this rise over its
# series-start baseline. The measured events were 256 KiB of zram, owned by
# no process; 1 MiB passes such events and still stops real reclaim.
: "${STUDY_SWAP_TOLERANCE_KIB:=1024}"

# Per-process time limits (seconds): Node/Python runs and C probes.
: "${STUDY_RUN_TIMEOUT_S:=600}"
: "${STUDY_C_TIMEOUT_S:=120}"

# Largest pool any exhaustion or capacity probe may create (owner ruling 5):
# a full pool holds ceil32(maxSize/3) of real memory on this host.
: "${STUDY_MAX_POOL_MIB:=8192}"

# CB2: seconds of idle after the last close before the process exits.
: "${STUDY_IDLE_AFTER_CLOSE_S:=10}"

export STUDY_QUIET_MEM_KIB STUDY_QUIET_LOAD1_MAX STUDY_GPU_IDLE_SAMPLES STUDY_QUIET_WAIT_S \
  STUDY_MEM_FLOOR_KIB STUDY_SWAP_TOLERANCE_KIB STUDY_RUN_TIMEOUT_S STUDY_C_TIMEOUT_S \
  STUDY_MAX_POOL_MIB STUDY_IDLE_AFTER_CLOSE_S
