#!/usr/bin/env bash
# scripts/gcp-lab/remote-build.sh — runs ON a lab VM created by
# scripts/gcp-cuda-lab.sh. Installs build tools, clones the public FathomDB repo
# at --ref, builds the workspace (CPU only), and writes a receipt to
# ~/fdb-lab-receipts/receipt.json whether or not the build succeeds.
set -euo pipefail

REF="" PROFILE="" RUN_ID=""
REPO_URL="https://github.com/fathomadb/fathomdb.git"
while [ "$#" -gt 0 ]; do
  case "$1" in
    --ref) REF="$2"; shift 2 ;;
    --profile) PROFILE="$2"; shift 2 ;;
    --run-id) RUN_ID="$2"; shift 2 ;;
    *) printf 'remote-build: unknown option %s\n' "$1" >&2; exit 2 ;;
  esac
done
[ -n "$REF" ] && [ -n "$PROFILE" ] && [ -n "$RUN_ID" ] || { echo 'remote-build: --ref, --profile, --run-id required' >&2; exit 2; }

OUT="$HOME/fdb-lab-receipts"
mkdir -p "$OUT"
# The log copy is best-effort; the receipt is the record.
# shellcheck disable=SC2312
exec > >(tee -a "$OUT/remote.log") 2>&1

meta() {
  curl -fsS -H 'Metadata-Flavor: Google' \
    "http://metadata.google.internal/computeMetadata/v1/instance/$1" 2>/dev/null || printf 'unknown'
}

started="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
export DEBIAN_FRONTEND=noninteractive
sudo apt-get update -q
sudo apt-get install -yq build-essential pkg-config git curl python3 python3-dev

if [ ! -x "$HOME/.cargo/bin/rustup" ]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
fi
# shellcheck source=/dev/null
. "$HOME/.cargo/env"

rm -rf "$HOME/src"
git clone --quiet --depth 1 --branch "$REF" "$REPO_URL" "$HOME/src"
cd "$HOME/src"
sha="$(git rev-parse HEAD)"
rustup toolchain install
toolchain="$(rustc --version)"

build_cmd=(cargo build --workspace --locked)
build_start="$(date +%s)"
build_status=0
"${build_cmd[@]}" || build_status=$?
build_seconds=$(( $(date +%s) - build_start ))

if command -v nvidia-smi >/dev/null 2>&1; then
  gpu="$(nvidia-smi --query-gpu=name,driver_version --format=csv,noheader 2>&1 || true)"
else
  gpu="nvidia-smi absent"
fi

machine_type="$(meta machine-type)"
zone="$(meta zone)"
image="$(meta image)"
kernel="$(uname -r)"
nproc_count="$(nproc)"
# shellcheck source=/dev/null
os_name="$(. /etc/os-release && printf '%s' "$PRETTY_NAME")"

RECEIPT_RUN_ID="$RUN_ID" RECEIPT_PROFILE="$PROFILE" RECEIPT_REF="$REF" RECEIPT_SHA="$sha" \
RECEIPT_REPO="$REPO_URL" RECEIPT_STARTED="$started" RECEIPT_TOOLCHAIN="$toolchain" \
RECEIPT_BUILD_CMD="${build_cmd[*]}" RECEIPT_BUILD_STATUS="$build_status" \
RECEIPT_BUILD_SECONDS="$build_seconds" RECEIPT_GPU="$gpu" \
RECEIPT_MACHINE_TYPE="$machine_type" RECEIPT_ZONE="$zone" RECEIPT_IMAGE="$image" \
RECEIPT_KERNEL="$kernel" RECEIPT_NPROC="$nproc_count" RECEIPT_OS="$os_name" \
python3 -I - "$OUT/receipt.json" <<'PY'
import json, os, sys, datetime
fields = {k[len("RECEIPT_"):].lower(): v for k, v in os.environ.items() if k.startswith("RECEIPT_")}
for key in ("build_status", "build_seconds", "nproc"):
    fields[key] = int(fields[key])
for key in ("machine_type", "zone", "image"):
    fields[key] = fields[key].rsplit("/", 1)[-1]
fields["finished"] = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
with open(sys.argv[1], "w") as fh:
    json.dump(dict(sorted(fields.items())), fh, indent=2)
    fh.write("\n")
PY
cat "$OUT/receipt.json"
exit "$build_status"
