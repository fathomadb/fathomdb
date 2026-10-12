#!/usr/bin/env bash
# Tests for scripts/gcp-lab/remote-driver.sh. The script normally runs on a lab
# VM; here every command that could touch this host's packages, kernel or
# driver is a stub first on PATH, and HOME is a temp dir.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
DRIVER="$ROOT/scripts/gcp-lab/remote-driver.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT
REAL_CAT="$(command -v cat)"

STUBS=(sudo apt-get modprobe nvidia-smi mokutil dpkg-query uname lspci lsmod dmesg grub-reboot cat)
mkdir -p "$TMP/bin"
for name in "${STUBS[@]}"; do
  cat >"$TMP/bin/$name" <<'STUB'
#!/usr/bin/env bash
name="${0##*/}"
printf '%s %s\n' "$name" "$*" >>"$STUB_LOG"
case "$name" in
  sudo)
    # Only ever run another stub; a real binary must never run as root here.
    [ -x "$STUB_BIN/$1" ] || exit 0
    exec "$STUB_BIN/$1" "${@:2}" ;;
  apt-get)
    if [ -n "${STUB_PREBUILT_FAIL:-}" ] && [[ "$*" == *linux-modules-nvidia-* ]]; then exit 100; fi ;;
  uname) printf '%s\n' "$STUB_KERNEL" ;;
  nvidia-smi)
    [ -z "${STUB_SMI_FAIL:-}" ] || exit 9
    case "$*" in
      *--query-gpu=driver_version*) printf '%s\n' "$STUB_DRIVER_VERSION" ;;
      *--query-gpu=*) printf 'Tesla T4, %s, 7.5\n' "$STUB_DRIVER_VERSION" ;;
    esac ;;
  mokutil) echo 'SecureBoot enabled' ;;
  dpkg-query)
    glob="${*: -1}"
    for pkg in $STUB_INSTALLED; do
      # shellcheck disable=SC2053 # the glob is the point
      if [[ "$pkg" == $glob ]]; then printf '%s=1.0 ' "$pkg"; fi
    done ;;
  cat)
    if [ "$1" = /proc/driver/nvidia/version ]; then
      echo "NVRM version: NVIDIA UNIX Open Kernel Module for x86_64  $STUB_DRIVER_VERSION"
      exit 0
    fi
    exec "$STUB_REAL_CAT" "$@" ;;
esac
exit 0
STUB
  chmod +x "$TMP/bin/$name"
done
export PATH="$TMP/bin:$PATH" STUB_BIN="$TMP/bin" STUB_LOG="$TMP/stub.log" STUB_REAL_CAT="$REAL_CAT"

fail() { echo "FAIL $*"; exit 1; }

# No real package, kernel or driver tool may be reachable before any case runs.
for name in "${STUBS[@]}"; do
  resolved="$(command -v "$name" || true)"
  [ "$resolved" = "$TMP/bin/$name" ] || fail "$name does not resolve to the stub"
done

RUN_STATUS=0
run_driver() {
  : >"$STUB_LOG"
  rm -rf "${TMP:?}/home"
  mkdir -p "$TMP/home"
  RUN_STATUS=0
  HOME="$TMP/home" bash "$DRIVER" "$@" || RUN_STATUS=$?
}
show() { echo "stub calls:"; "$REAL_CAT" "$STUB_LOG"; echo "driver.log:"; "$REAL_CAT" "$TMP/home/fdb-lab-receipts/driver.log" 2>/dev/null || true; }
expect_line() { grep -qxF -- "$1" "$STUB_LOG" || { show; fail "$2: missing line '$1'"; }; }
expect_none() { if grep -qF -- "$1" "$STUB_LOG"; then show; fail "$2: unexpected '$1'"; fi; }
expect_status() { [ "$RUN_STATUS" -eq "$1" ] || { show; fail "$2: exit $RUN_STATUS, expected $1"; }; }
expect_installs() {
  local n
  n="$(grep -c '^apt-get install' "$STUB_LOG" || true)"
  [ "$n" -eq "$1" ] || { show; fail "$2: $n apt-get installs, expected $1"; }
}
json_field() {
  python3 -I -c 'import json,sys; d = json.load(open(sys.argv[1])); print(json.dumps(d[sys.argv[2]]) if sys.argv[2] in d else "<missing>")' \
    "$TMP/home/fdb-lab-receipts/driver.json" "$1"
}
expect_json() {
  local got
  got="$(json_field "$1")"
  [ "$got" = "$2" ] || fail "$3: driver.json $1 is $got, expected $2"
}

INSTALL='apt-get install -yq --no-install-recommends'
export STUB_KERNEL=7.0.0-1014-gcp STUB_DRIVER_VERSION=615.71.09
export STUB_INSTALLED="linux-modules-nvidia-615-server-open-7.0.0-1014-gcp nvidia-headless-no-dkms-615-server-open nvidia-utils-615-server libnvidia-compute-615-server nvidia-utils-580-server"

# Default: the proprietary flavour, exactly as before --open existed.
STUB_DRIVER_VERSION=580.95.05 run_driver --driver-branch 580
expect_status 0 "default"
expect_line "$INSTALL linux-modules-nvidia-580-server-7.0.0-1014-gcp nvidia-headless-no-dkms-580-server nvidia-utils-580-server" "default prebuilt"
expect_line "modprobe nvidia" "default loads the module"
expect_installs 1 "default"
expect_json open false "default"
expect_json requested_kernel null "default"
expect_json kernel '"7.0.0-1014-gcp"' "default"
expect_json method '"prebuilt-module"' "default"

# --open: the open kernel module flavour, prebuilt for the running kernel.
run_driver --driver-branch 615 --open
expect_status 0 "open"
expect_line "$INSTALL linux-modules-nvidia-615-server-open-7.0.0-1014-gcp nvidia-headless-no-dkms-615-server-open nvidia-utils-615-server" "open prebuilt"
expect_line "modprobe nvidia" "open loads the module"
expect_json open true "open"
expect_json branch '"615"' "open"
expect_json version '"615.71.09"' "open"
expect_json packages '["linux-modules-nvidia-615-server-open-7.0.0-1014-gcp", "nvidia-headless-no-dkms-615-server-open", "nvidia-utils-615-server", "libnvidia-compute-615-server"]' "open packages"

# --open with no prebuilt module: DKMS builds the open module.
STUB_PREBUILT_FAIL=1 run_driver --driver-branch 615 --open
expect_status 0 "open dkms"
expect_line "$INSTALL linux-headers-7.0.0-1014-gcp nvidia-headless-615-server-open nvidia-utils-615-server" "open dkms fallback"
expect_json method '"dkms"' "open dkms"

# The fallback branch takes --open the same way.
STUB_DRIVER_VERSION=595.91.07 run_driver --driver-branch 595 --open
expect_status 0 "open 595"
expect_line "$INSTALL linux-modules-nvidia-595-server-open-7.0.0-1014-gcp nvidia-headless-no-dkms-595-server-open nvidia-utils-595-server" "open 595"

# --kernel equal to the running kernel behaves as without it.
run_driver --driver-branch 615 --open --kernel 7.0.0-1014-gcp
expect_status 0 "kernel already running"
expect_none "linux-image-" "kernel already running"
expect_none "grub-reboot" "kernel already running"
expect_line "modprobe nvidia" "kernel already running"
expect_json requested_kernel '"7.0.0-1014-gcp"' "kernel already running"

# --kernel newer than the running kernel: install the kernel and the driver
# built for it, point the next boot at it, and ask for the one reboot.
STUB_KERNEL=6.17.0-1009-gcp run_driver --driver-branch 615 --open --kernel 7.0.0-1014-gcp
expect_status 100 "kernel change"
expect_line "$INSTALL linux-image-7.0.0-1014-gcp" "kernel change installs the kernel"
expect_line "$INSTALL linux-modules-nvidia-615-server-open-7.0.0-1014-gcp nvidia-headless-no-dkms-615-server-open nvidia-utils-615-server" "kernel change driver"
expect_line "grub-reboot Advanced options for Ubuntu>Ubuntu, with Linux 7.0.0-1014-gcp" "kernel change selects the next boot"
expect_none "modprobe" "kernel change"
expect_none "6.17.0-1009-gcp" "kernel change"
kernel_line="$(grep -n -m1 -- 'linux-image-7.0.0-1014-gcp' "$STUB_LOG")"
driver_line="$(grep -n -m1 -- 'linux-modules-nvidia-615' "$STUB_LOG")"
[ "${kernel_line%%:*}" -lt "${driver_line%%:*}" ] || { show; fail "the kernel must install before the driver"; }

# The DKMS fallback builds against the requested kernel's headers.
STUB_KERNEL=6.17.0-1009-gcp STUB_PREBUILT_FAIL=1 run_driver --driver-branch 615 --open --kernel 7.0.0-1014-gcp
expect_status 100 "kernel change dkms"
expect_line "$INSTALL linux-headers-7.0.0-1014-gcp nvidia-headless-615-server-open nvidia-utils-615-server" "kernel change dkms"

# After the reboot, --verify-only refuses a kernel other than the requested one.
STUB_KERNEL=6.17.0-1009-gcp run_driver --driver-branch 615 --open --kernel 7.0.0-1014-gcp --verify-only
[ "$RUN_STATUS" -ne 0 ] && [ "$RUN_STATUS" -ne 100 ] || { show; fail "verify accepted the wrong kernel (exit $RUN_STATUS)"; }
for kernel in 7.0.0-1014-gcp 6.17.0-1009-gcp; do
  grep -qF "$kernel" "$TMP/home/fdb-lab-receipts/driver.log" \
    || { show; fail "verify on the wrong kernel must name $kernel"; }
done
[ -e "$TMP/home/fdb-lab-receipts/driver-diagnostics.txt" ] || fail "verify on the wrong kernel wrote no diagnostics"
[ ! -e "$TMP/home/fdb-lab-receipts/driver.json" ] || fail "verify on the wrong kernel wrote a receipt"
expect_installs 0 "verify"

# ... and on the requested kernel records it.
run_driver --driver-branch 615 --open --kernel 7.0.0-1014-gcp --verify-only
expect_status 0 "verify on the requested kernel"
expect_installs 0 "verify"
expect_json kernel '"7.0.0-1014-gcp"' "verify"
expect_json requested_kernel '"7.0.0-1014-gcp"' "verify"
expect_json open true "verify"
expect_json method '"verified-after-reboot"' "verify"

# Bad kernel names never reach apt-get.
for bad in 7.0.0-1014-generic '7.0.0-1014-gcp;reboot' ../7.0.0-1014-gcp ''; do
  run_driver --driver-branch 615 --open --kernel "$bad"
  [ "$RUN_STATUS" -eq 2 ] || fail "kernel '$bad' was accepted (exit $RUN_STATUS)"
  expect_none "apt-get" "kernel '$bad'"
done

echo 'All gcp-lab remote-driver tests passed'
