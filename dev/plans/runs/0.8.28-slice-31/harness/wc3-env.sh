#!/usr/bin/env bash
# 0.8.28 Slice 31 windchill3 guard and GPU selection (test-plan § 1.1).
# Sourced by every windchill3 run that can reach CUDA. The NVIDIA driver and
# /usr/local/cuda are never changed (Q4, Q6, Q11); the Quadro K620 is never
# run (Q22): CUDA_VISIBLE_DEVICES names the RTX 3090 at PCI 00000000:41:00.0
# by UUID, never `all`, never an index.
#
# Usage: GUARD_LOG=<file> . wc3-env.sh; wc3_guard before-<cell>; ...; wc3_guard after-<cell>
GUARD_LOG="${GUARD_LOG:?set GUARD_LOG to the guard log file}"
wc3_guard() {
  local driver="" cuda now line listing
  listing="$(nvidia-smi --query-gpu=driver_version --format=csv,noheader)" || return 3
  local -a drivers
  mapfile -t drivers <<<"$listing"
  for line in "${drivers[@]}"; do
    case ",$driver," in
      *",$line,"*) ;;
      *) driver="${driver:+$driver,}$line" ;;
    esac
  done
  cuda="$(readlink -f /usr/local/cuda)"
  now="$(date -u +%FT%TZ)"
  printf '%s %s driver=%s cuda=%s\n' "$now" "$1" "$driver" "$cuda" >>"$GUARD_LOG"
  if [ "$driver" != 580.178.04 ] || [ "$cuda" != /usr/local/cuda-12.6 ]; then
    echo "wc3 guard changed: driver=$driver cuda=$cuda; stop the slice" >&2
    return 3
  fi
}
RTX3090_UUID=""
WC3_LISTING="$(nvidia-smi --query-gpu=uuid,pci.bus_id --format=csv,noheader)" || WC3_LISTING=""
mapfile -t WC3_GPUS <<<"$WC3_LISTING"
for WC3_GPU in "${WC3_GPUS[@]}"; do
  if [ "${WC3_GPU#*, }" = "00000000:41:00.0" ]; then
    RTX3090_UUID="${WC3_GPU%%, *}"
  fi
done
if ! [[ "$RTX3090_UUID" =~ ^GPU-[0-9a-f-]{36}$ ]]; then
  echo "no RTX 3090 at 41:00.0" >&2
  # shellcheck disable=SC2317 # reached when executed rather than sourced
  return 3 2>/dev/null || exit 3
fi
export CUDA_DEVICE_ORDER=PCI_BUS_ID CUDA_VISIBLE_DEVICES="$RTX3090_UUID" FATHOMDB_CUDA_GPU_UUID="$RTX3090_UUID"
export CUDA_COMPUTE_CAP=75 CUDARC_CUDA_VERSION=12060 CUDA_PATH=/usr/local/cuda-12.6
export PATH="/usr/local/cuda-12.6/bin:$PATH"
