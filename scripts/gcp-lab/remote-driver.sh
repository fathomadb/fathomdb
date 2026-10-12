#!/usr/bin/env bash
# scripts/gcp-lab/remote-driver.sh — runs ON a GPU lab VM created by
# scripts/gcp-cuda-lab.sh. Installs one pinned NVIDIA driver branch from
# Ubuntu's archive (Canonical-signed, so it loads under Secure Boot), loads
# it, and writes ~/fdb-lab-receipts/driver.json.
#
# Options: --driver-branch NNN (required); --open selects the open kernel
# module flavour (-server-open), which branches from 595 on need for Turing;
# --kernel <release> installs that GCP kernel first when the VM runs another
# one, puts the driver built for it alongside, and asks for a reboot;
# --verify-only checks a driver installed before a reboot.
#
# Exit codes: 0 driver loaded and on the requested branch; 100 installed but
# not usable until a reboot (measured on a T4: the module loads and binds, but
# the /dev/nvidia* nodes are missing because --no-install-recommends leaves
# out nvidia-modprobe); anything else is a failure, with diagnostics in
# driver-diagnostics.txt.
set -euo pipefail

BRANCH="" VERIFY_ONLY=0 OPEN=0 FLAVOUR="" REQUESTED_KERNEL=""
KERNEL_RE='^[0-9]+\.[0-9]+\.[0-9]+-[0-9]+-gcp$'
while [ "$#" -gt 0 ]; do
  case "$1" in
    --driver-branch) BRANCH="$2"; shift 2 ;;
    --verify-only) VERIFY_ONLY=1; shift ;;
    --open) OPEN=1 FLAVOUR="-open"; shift ;;
    --kernel)
      [[ "${2:-}" =~ $KERNEL_RE ]] || { printf 'remote-driver: --kernel must look like 7.0.0-1014-gcp (got %s)\n' "${2:-}" >&2; exit 2; }
      REQUESTED_KERNEL="$2"; shift 2 ;;
    *) printf 'remote-driver: unknown option %s\n' "$1" >&2; exit 2 ;;
  esac
done
[[ "$BRANCH" =~ ^[0-9]{3}$ ]] || { echo 'remote-driver: --driver-branch NNN required' >&2; exit 2; }

OUT="$HOME/fdb-lab-receipts"
mkdir -p "$OUT"
# Write straight to the file: a tee process substitution loses the last lines
# when the script exits.
exec >>"$OUT/driver.log" 2>&1

diagnose() {
  {
    echo "== lsmod"; lsmod | grep -E 'nvidia|nouveau' || echo "(no nvidia or nouveau module)"
    echo "== /dev/nvidia*"; ls -l /dev/nvidia* 2>&1 || true
    echo "== nvidia-smi"; nvidia-smi 2>&1 || true
    echo "== lspci"; lspci -nnk -d 10de: 2>&1 || true
    echo "== dmesg"; sudo dmesg | grep -iE 'nvidia|nouveau|NVRM' | tail -n 60 || true
    echo "== secure boot"; mokutil --sb-state 2>&1 || true
  } >"$OUT/driver-diagnostics.txt" 2>&1
}

kernel="$(uname -r)"
target_kernel="${REQUESTED_KERNEL:-$kernel}"
method="verified-after-reboot"
if [ "$VERIFY_ONLY" -eq 1 ] && [ "$target_kernel" != "$kernel" ]; then
  diagnose
  echo "remote-driver: running kernel $kernel, but $target_kernel was requested; the reboot did not boot it"
  exit 1
fi
if [ "$VERIFY_ONLY" -eq 0 ]; then
  export DEBIAN_FRONTEND=noninteractive
  sudo apt-get update -q
  if [ "$target_kernel" != "$kernel" ]; then
    # linux-image-* depends on its linux-modules-*. --no-install-recommends
    # keeps out its grub-pc recommendation, which would fight the image's
    # EFI GRUB.
    sudo apt-get install -yq --no-install-recommends "linux-image-$target_kernel"
  fi
  # Prefer the prebuilt module for the target kernel; fall back to DKMS.
  if sudo apt-get install -yq --no-install-recommends \
      "linux-modules-nvidia-$BRANCH-server$FLAVOUR-$target_kernel" \
      "nvidia-headless-no-dkms-$BRANCH-server$FLAVOUR" "nvidia-utils-$BRANCH-server"; then
    method="prebuilt-module"
  else
    sudo apt-get install -yq --no-install-recommends "linux-headers-$target_kernel" \
      "nvidia-headless-$BRANCH-server$FLAVOUR" "nvidia-utils-$BRANCH-server"
    method="dkms"
  fi
  if [ "$target_kernel" != "$kernel" ]; then
    # The default entry boots the newest installed kernel, which need not be
    # the requested one, so make the next boot (only) take it by title.
    # --verify-only catches a boot that ignored this.
    if ! sudo grub-reboot "Advanced options for Ubuntu>Ubuntu, with Linux $target_kernel"; then
      echo "remote-driver: grub-reboot failed; the next boot takes the default kernel"
    fi
    echo "remote-driver: kernel $target_kernel installed; reboot required"
    exit 100
  fi
  if ! sudo modprobe nvidia || ! nvidia-smi >/dev/null 2>&1; then
    diagnose
    echo "remote-driver: driver not usable yet; reboot required"
    exit 100
  fi
fi

if ! nvidia-smi >/dev/null 2>&1; then
  diagnose
  echo "remote-driver: nvidia-smi still fails after reboot; see driver-diagnostics.txt"
  exit 1
fi

smi="$(nvidia-smi --query-gpu=name,driver_version,compute_cap --format=csv,noheader)"
echo "nvidia-smi: $smi"
driver_version="$(nvidia-smi --query-gpu=driver_version --format=csv,noheader)"
driver_version="${driver_version%%$'\n'*}"
proc_version="$(cat /proc/driver/nvidia/version)"
proc_version="${proc_version%%$'\n'*}"
sb_state="$(mokutil --sb-state 2>&1 || true)"
packages="$(dpkg-query -W -f='${Package}=${Version} ' "*nvidia*$BRANCH*" 2>/dev/null || true)"

DRV_BRANCH="$BRANCH" DRV_VERSION="$driver_version" DRV_SMI="$smi" DRV_PROC="$proc_version" \
DRV_KERNEL="$kernel" DRV_METHOD="$method" DRV_SECURE_BOOT="$sb_state" DRV_PACKAGES="$packages" \
DRV_OPEN="$OPEN" DRV_REQUESTED_KERNEL="$REQUESTED_KERNEL" \
python3 -I - "$OUT/driver.json" <<'PY'
import json, os, sys
fields = {k[len("DRV_"):].lower(): v for k, v in os.environ.items() if k.startswith("DRV_")}
fields["packages"] = fields["packages"].split()
fields["open"] = fields["open"] == "1"
fields["requested_kernel"] = fields["requested_kernel"] or None
with open(sys.argv[1], "w") as fh:
    json.dump(dict(sorted(fields.items())), fh, indent=2)
    fh.write("\n")
PY
cat "$OUT/driver.json"

case "$driver_version" in
  "$BRANCH".*) exit 0 ;;
  *) echo "remote-driver: loaded driver $driver_version is not branch $BRANCH"; exit 1 ;;
esac
