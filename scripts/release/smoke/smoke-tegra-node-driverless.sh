#!/usr/bin/env bash
# Driverless AArch64 import of the Jetson Tegra Node package
# (ADR-0.8.27-jetson-tegra-node-addon-distribution, decisions 5 and 6).
#
# Runs on a hosted, non-Tegra Linux ARM64 host without a CUDA driver. Inside
# a network-less linux/arm64 container with no GPU, it installs the
# Jetson-built tarballs into fresh consumers with lifecycle scripts disabled
# and proves that:
#   - the Tegra addon registers and opens an engine without a CUDA driver,
#     with early CUDA init at its default and with FATHOMDB_CUDA_EARLY_INIT=off;
#   - the main package's loader ignores the installed Tegra package on this
#     non-Tegra host, both with a generic linux-arm64-gnu stand-in co-installed
#     and without one.
# tegra_node_driverless_verdict.mjs decides the result from the retained
# rows; a missing row is a failure.
#
# Nothing here publishes. Exit 2 is a usage error; exit 1 is any failure,
# including a host that cannot give this proof.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
# shellcheck source=../cuda-artifact-contract.sh
. "$REPO_ROOT/scripts/release/cuda-artifact-contract.sh"

usage() {
  cat >&2 <<USAGE
usage: $(basename "$0") --main-tarball <tgz> --tegra-tarball <tgz>
         --candidate-sha <40-hex> --evidence-dir <new-dir>
USAGE
}
usage_error() {
  printf 'smoke-tegra-node-driverless: %s\n' "$1" >&2
  usage
  exit 2
}
fail() {
  printf 'smoke-tegra-node-driverless: %s\n' "$1" >&2
  exit 1
}

MAIN_TARBALL='' TEGRA_TARBALL='' CANDIDATE_SHA='' EVIDENCE_DIR=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --main-tarball | --tegra-tarball | --candidate-sha | --evidence-dir)
      [ "$#" -ge 2 ] || usage_error "$1 needs a value"
      case "$1" in
        --main-tarball) MAIN_TARBALL="$2" ;;
        --tegra-tarball) TEGRA_TARBALL="$2" ;;
        --candidate-sha) CANDIDATE_SHA="$2" ;;
        --evidence-dir) EVIDENCE_DIR="$2" ;;
        *) usage_error "unhandled argument $1" ;;
      esac
      shift 2
      ;;
    *) usage_error "unknown argument: $1" ;;
  esac
done
[ -n "$MAIN_TARBALL" ] && [ -n "$TEGRA_TARBALL" ] || usage_error '--main-tarball and --tegra-tarball are required'
[[ "$CANDIDATE_SHA" =~ ^[0-9a-f]{40}$ ]] || usage_error 'candidate SHA must be 40 lowercase hexadecimal characters'
[ -n "$EVIDENCE_DIR" ] || usage_error '--evidence-dir is required'

[ ! -e "$EVIDENCE_DIR" ] || fail "evidence path must be new: $EVIDENCE_DIR"
for tarball in "$MAIN_TARBALL" "$TEGRA_TARBALL"; do
  [ -f "$tarball" ] || fail "tarball is absent: $tarball"
done

# --- Only a driverless, non-Tegra AArch64 host can give this proof ----------
# The container hides some host facts, so the host itself is checked first:
# a pass here must never come from a Jetson or a GPU host.
host_os="$(uname -s)"
host_arch="$(uname -m)"
if [ "$host_os" != Linux ] || [ "$host_arch" != aarch64 ]; then
  fail "not a Linux AArch64 host ($host_os $host_arch)"
fi
if grep -aqs 'nvidia,tegra' /proc/device-tree/compatible || [ -e /etc/nv_tegra_release ]; then
  fail 'Tegra-family host: the driverless import must run on a non-Tegra AArch64 runner'
fi
if ! ld_cache="$(ldconfig -p 2>/dev/null)"; then
  ld_cache=''
fi
if grep -q 'libcuda\.so' <<<"$ld_cache"; then
  fail 'CUDA driver library is present: the import must run without a CUDA driver'
fi
for smi in /usr/bin/nvidia-smi /usr/sbin/nvidia-smi /usr/local/bin/nvidia-smi; do
  [ ! -e "$smi" ] || fail "nvidia-smi is present at $smi: the import must run without a CUDA driver"
done
command -v docker >/dev/null 2>&1 || fail 'docker is required to run the network-less driverless container'

MAIN_TARBALL="$(realpath -e -- "$MAIN_TARBALL")"
TEGRA_TARBALL="$(realpath -e -- "$TEGRA_TARBALL")"
HOST_UID="$(id -u)"
HOST_GID="$(id -g)"
mkdir -p "$EVIDENCE_DIR"
EVIDENCE_DIR="$(cd "$EVIDENCE_DIR" && pwd)"
INPUTS="$EVIDENCE_DIR/inputs"
mkdir -p "$INPUTS"
cp "$MAIN_TARBALL" "$INPUTS/main.tgz"
cp "$TEGRA_TARBALL" "$INPUTS/tegra.tgz"
printf 'image=%s\n' "$CUDA_TEGRA_DRIVERLESS_NODE_IMAGE" >"$EVIDENCE_DIR/image.txt"

# No GPU, no NVIDIA runtime and no network: the container sees only the
# tarballs (read-only), the smoke's own scripts (read-only) and the evidence
# directory.
set +e
docker run --rm --platform linux/arm64 --network none \
  --user "$HOST_UID:$HOST_GID" \
  --env HOME=/tmp/home \
  --env CANDIDATE_SHA="$CANDIDATE_SHA" \
  --mount "type=bind,src=$INPUTS,dst=/inputs,readonly" \
  --mount "type=bind,src=$SCRIPT_DIR,dst=/smoke,readonly" \
  --mount "type=bind,src=$EVIDENCE_DIR,dst=/evidence" \
  "$CUDA_TEGRA_DRIVERLESS_NODE_IMAGE" \
  bash /smoke/tegra_node_driverless_probe.sh 2>&1 | tee "$EVIDENCE_DIR/probe.log"
status="${PIPESTATUS[0]}"
set -e
[ "$status" -eq 0 ] || fail "driverless import failed (exit $status); see $EVIDENCE_DIR/summary.json"
printf 'smoke-tegra-node-driverless: PASS (%s)\n' "$EVIDENCE_DIR/summary.json"
