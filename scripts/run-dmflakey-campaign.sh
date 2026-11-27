#!/usr/bin/env bash
# scripts/run-dmflakey-campaign.sh — disposable QEMU/KVM dm-flakey power-loss
# durability campaign orchestrator for AC32-DMFLAKEY-01..04 (ADR-0.8.32 #101).
#
# Runs completely in user space over /dev/kvm (zero host sudo, zero host
# mutation). All VM images, overlays, cidata ISOs, raw fault disks, oracle
# logs, and campaign JSON receipts live strictly under --vm-dir (outside git).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

VM_DIR="/mnt/data/worktrees/fathomdb-0.8.32-dmflakey-vm"
HARNESS_BIN="$REPO_ROOT/target/release/fathomdb-dmflakey-harness"
CLI_BIN="$REPO_ROOT/target/release/fathomdb"
SSH_PORT="22222"
RAM_MB="2048"
CPUS="2"
DATA_DISK_MB="256"
SCENARIOS_FILTER="all"
DRY_RUN=0
SELF_TEST=0

QEMU_PID=""
ACTIVE_RUN_DIR=""

ALL_CASES=(
  "normal_active_commit_poweroff"
  "normal_wal_accum_drop_writes"
  "normal_checkpoint_erasure_drop_writes"
  "normal_reopen_replay_double_cut"
  "normal_erasure_lock_fault"
  "full_active_commit_poweroff"
  "full_wal_accum_poweroff"
  "full_checkpoint_erasure_poweroff"
  "full_reopen_replay_double_cut"
  "full_erasure_lock_fault"
  "full_error_writes_eio"
)

usage() {
  cat <<'EOF'
Usage: scripts/run-dmflakey-campaign.sh [options]

Options:
  --vm-dir <path>         Staging dir outside git (default: /mnt/data/worktrees/fathomdb-0.8.32-dmflakey-vm)
  --harness-bin <path>    Path to fathomdb-dmflakey-harness release binary
  --cli-bin <path>        Path to fathomdb CLI release binary
  --ssh-port <port>       Local hostfwd TCP port for guest SSH (default: 22222)
  --ram-mb <mb>           Guest RAM in MiB (default: 2048)
  --cpus <n>              Guest vCPU count (default: 2)
  --data-disk-mb <mb>     Raw fault-test virtio-blk disk size in MiB (default: 256)
  --scenarios <csv|all>   Comma-separated case names or 'all' (default: all)
  --dry-run               Validate host tools and print campaign plan without booting QEMU
  --self-test             Run internal script validation checks without booting QEMU
  -h, --help              Show this help text
EOF
}

log() {
  local ts
  ts="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  printf '[%s] %s\n' "$ts" "$*"
}

die() {
  printf 'FAIL run-dmflakey-campaign: %s\n' "$*" >&2
  exit 1
}

cleanup_qemu() {
  if [ -n "$QEMU_PID" ]; then
    if kill -0 "$QEMU_PID" 2>/dev/null; then
      kill -9 "$QEMU_PID" 2>/dev/null || true
      wait "$QEMU_PID" 2>/dev/null || true
    fi
    QEMU_PID=""
  fi
}

on_exit() {
  cleanup_qemu
}
trap on_exit EXIT INT TERM

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --vm-dir)
        [ "$#" -ge 2 ] || die "--vm-dir requires a value"
        VM_DIR="$2"
        shift 2
        ;;
      --harness-bin)
        [ "$#" -ge 2 ] || die "--harness-bin requires a value"
        HARNESS_BIN="$2"
        shift 2
        ;;
      --cli-bin)
        [ "$#" -ge 2 ] || die "--cli-bin requires a value"
        CLI_BIN="$2"
        shift 2
        ;;
      --ssh-port)
        [ "$#" -ge 2 ] || die "--ssh-port requires a value"
        SSH_PORT="$2"
        shift 2
        ;;
      --ram-mb)
        [ "$#" -ge 2 ] || die "--ram-mb requires a value"
        RAM_MB="$2"
        shift 2
        ;;
      --cpus)
        [ "$#" -ge 2 ] || die "--cpus requires a value"
        CPUS="$2"
        shift 2
        ;;
      --data-disk-mb)
        [ "$#" -ge 2 ] || die "--data-disk-mb requires a value"
        DATA_DISK_MB="$2"
        shift 2
        ;;
      --scenarios)
        [ "$#" -ge 2 ] || die "--scenarios requires a value"
        SCENARIOS_FILTER="$2"
        shift 2
        ;;
      --dry-run)
        DRY_RUN=1
        shift
        ;;
      --self-test)
        SELF_TEST=1
        shift
        ;;
      -h | --help)
        usage
        exit 0
        ;;
      *)
        die "unknown argument: $1"
        ;;
    esac
  done
}

require_host_tools() {
  local tool
  for tool in qemu-system-x86_64 qemu-img xorriso ssh-keygen ssh scp python3 sha256sum; do
    command -v "$tool" >/dev/null 2>&1 || die "required host tool not found: $tool"
  done
  if [ "$DRY_RUN" -eq 0 ] && [ "$SELF_TEST" -eq 0 ]; then
    [ -r /dev/kvm ] && [ -w /dev/kvm ] || die "/dev/kvm is not readable/writable by current user"
  fi
}

select_cases() {
  SELECTED_CASES=()
  if [ "$SCENARIOS_FILTER" = "all" ]; then
    SELECTED_CASES=("${ALL_CASES[@]}")
    return 0
  fi
  local raw_list item known candidate
  IFS=',' read -r -a raw_list <<<"$SCENARIOS_FILTER"
  for item in "${raw_list[@]}"; do
    known=0
    for candidate in "${ALL_CASES[@]}"; do
      if [ "$item" = "$candidate" ]; then
        known=1
        break
      fi
    done
    [ "$known" -eq 1 ] || die "unknown scenario case in --scenarios: $item"
    SELECTED_CASES+=("$item")
  done
  [ "${#SELECTED_CASES[@]}" -gt 0 ] || die "--scenarios resolved to zero cases"
}

run_self_test() {
  require_host_tools
  select_cases
  [ "${#SELECTED_CASES[@]}" -eq 11 ] || die "self-test expected 11 campaign cases, got ${#SELECTED_CASES[@]}"
  python3 -c '
import json
cap = json.dumps({"execute": "qmp_capabilities"})
quit_cmd = json.dumps({"execute": "quit"})
assert "qmp_capabilities" in cap and "quit" in quit_cmd
' || die "self-test QMP JSON check failed"
  log "SELF_TEST_OK cases=${#SELECTED_CASES[@]} vm_dir=$VM_DIR"
}

run_dry_run() {
  require_host_tools
  select_cases
  log "DRY_RUN vm_dir=$VM_DIR harness_bin=$HARNESS_BIN cli_bin=$CLI_BIN ssh_port=$SSH_PORT ram_mb=$RAM_MB cpus=$CPUS data_disk_mb=$DATA_DISK_MB"
  local case_name
  for case_name in "${SELECTED_CASES[@]}"; do
    printf '  case: %s\n' "$case_name"
  done
}

verify_cloud_image() {
  local img="$VM_DIR/noble-server-cloudimg-amd64.img"
  local sums="$VM_DIR/SHA256SUMS"
  [ -f "$img" ] || die "cloud image missing: $img"
  [ -f "$sums" ] || die "SHA256SUMS missing: $sums"

  local expected sum_line actual
  expected="$(awk '$2 ~ /noble-server-cloudimg-amd64\.img$/ { print $1; exit }' "$sums")"
  [ -n "$expected" ] || die "noble-server-cloudimg-amd64.img entry missing from $sums"
  sum_line="$(sha256sum "$img")" || die "sha256sum failed for $img"
  read -r actual _ <<<"$sum_line"
  if [ "$expected" != "$actual" ]; then
    die "SHA-256 mismatch for $img: expected=$expected actual=$actual"
  fi
  log "Verified Ubuntu 24.04 Noble cloud image SHA-256=$actual"
}

prepare_ssh_and_cidata() {
  local key_path="$VM_DIR/id_ed25519"
  local pub_key_path="${key_path}.pub"
  local seed_dir="$VM_DIR/seed"
  local cidata_iso="$VM_DIR/cidata.iso"

  mkdir -p "$seed_dir" "$VM_DIR/results/cases" "$VM_DIR/work"
  if [ ! -f "$key_path" ]; then
    ssh-keygen -q -t ed25519 -N "" -f "$key_path" -C "fathomdb-dmflakey-campaign"
  fi
  chmod 0600 "$key_path"

  local pubkey
  pubkey="$(cat "$pub_key_path")"

  cat >"$seed_dir/meta-data" <<'EOF'
instance-id: fathomdb-dmflakey-0832
local-hostname: fathomdb-dmflakey
EOF

  cat >"$seed_dir/user-data" <<EOF
#cloud-config
users:
  - name: ubuntu
    sudo: ALL=(ALL) NOPASSWD:ALL
    groups: sudo
    shell: /bin/bash
    lock_passwd: true
    ssh_authorized_keys:
      - ${pubkey}
ssh_pwauth: false
package_update: false
package_upgrade: false
EOF

  xorriso -as mkisofs \
    -quiet \
    -output "$cidata_iso" \
    -volid cidata \
    -joliet \
    -rock \
    "$seed_dir/user-data" \
    "$seed_dir/meta-data"
  log "Generated NoCloud seed ISO at $cidata_iso"
}

pick_free_ssh_port() {
  local port
  port="$(python3 - "$SSH_PORT" <<'PY'
import socket, sys
preferred = int(sys.argv[1])
for candidate in [preferred, 0]:
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        s.bind(("127.0.0.1", candidate))
        print(s.getsockname()[1])
        s.close()
        sys.exit(0)
    except OSError:
        s.close()
sys.exit(1)
PY
)"
  SSH_PORT="$port"
}

guest_ssh() {
  ssh \
    -i "$VM_DIR/id_ed25519" \
    -p "$SSH_PORT" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    -o LogLevel=ERROR \
    -o ConnectTimeout=5 \
    -o BatchMode=yes \
    ubuntu@127.0.0.1 \
    "$@"
}

guest_scp_to() {
  local src="$1"
  local dst="$2"
  scp \
    -i "$VM_DIR/id_ed25519" \
    -P "$SSH_PORT" \
    -o StrictHostKeyChecking=no \
    -o UserKnownHostsFile=/dev/null \
    -o LogLevel=ERROR \
    -o ConnectTimeout=5 \
    -o BatchMode=yes \
    "$src" "ubuntu@127.0.0.1:$dst"
}

start_vm() {
  local root_qcow2="$1"
  local data_disk="$2"
  local oracle_log="$3"
  local qmp_sock="$4"
  local console_log="$5"

  rm -f "$qmp_sock"
  : >"$oracle_log"
  : >"$console_log"

  local qemu_args=(
    -enable-kvm
    -machine q35
    -cpu host
    -m "$RAM_MB"
    -smp "$CPUS"
    -nographic
    -serial "file:$console_log"
    -monitor none
    -qmp "unix:$qmp_sock,server=on,wait=off"
    -drive "if=none,id=root0,file=$root_qcow2,format=qcow2,cache=writeback"
    -device "virtio-blk-pci,drive=root0,bootindex=1"
    -drive "if=virtio,file=$VM_DIR/cidata.iso,format=raw,readonly=on"
    -device virtio-serial-pci
    -chardev "file,id=oracle,path=$oracle_log"
    -device "virtserialport,chardev=oracle,name=oracle.0"
    -netdev "user,id=net0,hostfwd=tcp:127.0.0.1:${SSH_PORT}-:22"
    -device "virtio-net-pci,netdev=net0"
  )
  if [ -n "$data_disk" ]; then
    qemu_args+=(
      -drive "if=none,id=data0,file=$data_disk,format=raw,cache=none"
      -device "virtio-blk-pci,drive=data0,serial=fathomdb-flakey"
    )
  fi

  qemu-system-x86_64 "${qemu_args[@]}" </dev/null >/dev/null 2>&1 &
  QEMU_PID=$!
}

wait_for_ssh() {
  local timeout_secs="${1:-180}"
  local elapsed=0
  while [ "$elapsed" -lt "$timeout_secs" ]; do
    if [ -n "$QEMU_PID" ] && ! kill -0 "$QEMU_PID" 2>/dev/null; then
      die "QEMU process exited prematurely while waiting for SSH"
    fi
    if guest_ssh "true" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
    elapsed=$((elapsed + 1))
  done
  die "timed out after ${timeout_secs}s waiting for guest SSH on port $SSH_PORT"
}

qmp_hard_poweroff() {
  local qmp_sock="$1"
  if [ -S "$qmp_sock" ]; then
    python3 - "$qmp_sock" <<'PY' || true
import socket, sys
sock_path = sys.argv[1]
s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
s.settimeout(3.0)
s.connect(sock_path)
s.recv(4096)
s.sendall(b'{"execute":"qmp_capabilities"}\n')
s.recv(4096)
s.sendall(b'{"execute":"quit"}\n')
s.close()
PY
  fi

  local waited=0
  while [ -n "$QEMU_PID" ] && kill -0 "$QEMU_PID" 2>/dev/null && [ "$waited" -lt 20 ]; do
    sleep 0.1
    waited=$((waited + 1))
  done
  cleanup_qemu
}

graceful_shutdown_bootstrap() {
  guest_ssh "sudo poweroff" >/dev/null 2>&1 || true
  local waited=0
  while [ -n "$QEMU_PID" ] && kill -0 "$QEMU_PID" 2>/dev/null && [ "$waited" -lt 60 ]; do
    sleep 1
    waited=$((waited + 1))
  done
  cleanup_qemu
}

prepare_guest_template() {
  local base_img="$VM_DIR/noble-server-cloudimg-amd64.img"
  local prepared_root="$VM_DIR/root-prepared.qcow2"
  local stamp="$VM_DIR/root-prepared.sha256"
  local raw_sums hash_line bin_hash
  raw_sums="$(sha256sum "$HARNESS_BIN" "$CLI_BIN")" || die "sha256sum failed for campaign binaries"
  hash_line="$(sha256sum <<<"$raw_sums")" || die "sha256sum digest failed for campaign binaries"
  read -r bin_hash _ <<<"$hash_line"

  if [ -f "$prepared_root" ] && [ -f "$stamp" ]; then
    local prev_hash
    prev_hash="$(cat "$stamp")"
    if [ "$prev_hash" = "$bin_hash" ]; then
      log "Reusing prepared guest root overlay $prepared_root (binaries unchanged)"
      return 0
    fi
  fi

  log "Bootstrapping prepared guest root overlay $prepared_root"
  rm -f "$prepared_root"
  qemu-img create -q -f qcow2 -F qcow2 -b "$base_img" "$prepared_root"

  local boot_dir="$VM_DIR/work/bootstrap"
  mkdir -p "$boot_dir"
  start_vm "$prepared_root" "" "$boot_dir/oracle.log" "$boot_dir/qmp.sock" "$boot_dir/console.log"
  wait_for_ssh 240

  guest_scp_to "$HARNESS_BIN" "/tmp/fathomdb-dmflakey-harness"
  guest_scp_to "$CLI_BIN" "/tmp/fathomdb"

  guest_ssh 'bash -s' <<'REMOTE'
set -euo pipefail
sudo install -m 0755 /tmp/fathomdb-dmflakey-harness /usr/local/bin/fathomdb-dmflakey-harness
sudo install -m 0755 /tmp/fathomdb /usr/local/bin/fathomdb
rm -f /tmp/fathomdb-dmflakey-harness /tmp/fathomdb
/usr/local/bin/fathomdb-dmflakey-harness --help >/dev/null
/usr/local/bin/fathomdb --version >/dev/null

need_apt=0
if ! command -v dmsetup >/dev/null 2>&1; then
  need_apt=1
fi
if ! sudo modprobe dm-flakey 2>/dev/null; then
  need_apt=1
fi
if [ "$need_apt" -eq 1 ]; then
  krel="$(uname -r)"
  sudo apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq dmsetup "linux-modules-extra-${krel}"
fi
sudo modprobe dm-flakey
targets="$(sudo dmsetup targets)"
printf '%s\n' "$targets" | grep -m1 'flakey' >/dev/null
sudo sync
REMOTE

  graceful_shutdown_bootstrap
  printf '%s\n' "$bin_hash" >"$stamp"
  log "Prepared guest root overlay ready with dm-flakey + fathomdb binaries"
}

create_fresh_data_disk() {
  local disk_path="$1"
  rm -f "$disk_path"
  qemu-img create -q -f raw "$disk_path" "${DATA_DISK_MB}M"
}

create_ephemeral_root() {
  local run_root="$1"
  rm -f "$run_root"
  qemu-img create -q -f qcow2 -F qcow2 -b "$VM_DIR/root-prepared.qcow2" "$run_root"
}

wait_for_oracle_marker() {
  local oracle_log="$1"
  local marker="$2"
  local timeout_secs="${3:-60}"
  local ticks=$((timeout_secs * 20))
  local i=0
  while [ "$i" -lt "$ticks" ]; do
    if grep -F "$marker" "$oracle_log" >/dev/null 2>&1; then
      return 0
    fi
    if [ -n "$QEMU_PID" ] && ! kill -0 "$QEMU_PID" 2>/dev/null; then
      return 1
    fi
    sleep 0.05
    i=$((i + 1))
  done
  return 1
}

guest_setup_flakey_mount() {
  local format_disk="$1"
  guest_ssh "bash -s -- '$format_disk'" <<'REMOTE'
set -euo pipefail
format_disk="$1"
sudo modprobe dm-flakey
sudo udevadm settle
dev="$(readlink -f /dev/disk/by-id/virtio-fathomdb-flakey)"
[ -b "$dev" ]
sectors="$(sudo blockdev --getsz "$dev")"
if [ -b /dev/mapper/flakey-data ]; then
  sudo umount /mnt/flakey 2>/dev/null || true
  sudo dmsetup remove flakey-data
fi
sudo dmsetup create flakey-data --table "0 ${sectors} flakey ${dev} 0 3600 0"
sudo mkdir -p /mnt/flakey
if [ "$format_disk" = "format" ]; then
  sudo mkfs.ext4 -q -F /dev/mapper/flakey-data
  sudo mount -o data=ordered,barrier=1 /dev/mapper/flakey-data /mnt/flakey
  sudo chown ubuntu:ubuntu /mnt/flakey
else
  rc=0
  sudo fsck.ext4 -p /dev/mapper/flakey-data || rc=$?
  if [ "$rc" -ge 4 ]; then
    exit "$rc"
  fi
  sudo mount -o data=ordered,barrier=1 /dev/mapper/flakey-data /mnt/flakey
fi
sudo chmod 0666 /dev/virtio-ports/oracle.0
REMOTE
}

guest_switch_flakey_mode() {
  local feature="$1"
  guest_ssh "bash -s -- '$feature'" <<'REMOTE'
set -euo pipefail
feature="$1"
dev="$(readlink -f /dev/disk/by-id/virtio-fathomdb-flakey)"
sectors="$(sudo blockdev --getsz "$dev")"
sudo dmsetup suspend --nolockfs --noflush flakey-data
sudo dmsetup load flakey-data --table "0 ${sectors} flakey ${dev} 0 0 3600 1 ${feature}"
sudo dmsetup resume flakey-data
REMOTE
}

verify_after_reboot() {
  local case_id="$1"
  local sync_mode="$2"
  local scenario="$3"
  local data_disk="$4"
  local oracle_log="$5"
  local prev_oracle_log="$6"
  local allow_drop_rollback="$7"
  local run_dir="$8"

  local verify_root="$run_dir/verify-root.qcow2"
  create_ephemeral_root "$verify_root"
  start_vm "$verify_root" "$data_disk" "$run_dir/verify-oracle.log" "$run_dir/verify-qmp.sock" "$run_dir/verify-console.log"
  wait_for_ssh 120

  guest_scp_to "$oracle_log" "/tmp/oracle.log"
  local extra_flags=""
  if [ -n "$prev_oracle_log" ]; then
    guest_scp_to "$prev_oracle_log" "/tmp/prev-oracle.log"
    extra_flags="$extra_flags --prev-oracle-log /tmp/prev-oracle.log"
  fi
  if [ "$allow_drop_rollback" = "1" ]; then
    extra_flags="$extra_flags --allow-drop-writes-rollback"
  fi

  local guest_out
  guest_out="$(guest_ssh "bash -s -- '$sync_mode' '$scenario' '$extra_flags'" <<'REMOTE'
set -euo pipefail
sync_mode="$1"
scenario="$2"
extra_flags="$3"

sudo udevadm settle
dev="$(readlink -f /dev/disk/by-id/virtio-fathomdb-flakey)"
[ -b "$dev" ]
fsck_rc=0
sudo fsck.ext4 -p "$dev" >/tmp/fsck.out 2>&1 || fsck_rc=$?
if [ "$fsck_rc" -ge 4 ]; then
  cat /tmp/fsck.out >&2
  exit "$fsck_rc"
fi
sudo mkdir -p /mnt/flakey
sudo mount -o data=ordered,barrier=1 "$dev" /mnt/flakey

# shellcheck disable=SC2086
verify_out="$(/usr/local/bin/fathomdb-dmflakey-harness verify \
  --db /mnt/flakey/fathom.db \
  --sync-mode "$sync_mode" \
  --scenario "$scenario" \
  --oracle-log /tmp/oracle.log \
  $extra_flags)"
printf '%s\n' "$verify_out"

check_out="$(/usr/local/bin/fathomdb doctor check-integrity --full --json /mnt/flakey/fathom.db)"
printf 'DOCTOR_CHECK_INTEGRITY %s\n' "$check_out"

dpi_out="$(/usr/local/bin/fathomdb doctor data-plane-integrity --json /mnt/flakey/fathom.db)"
printf 'DOCTOR_DATA_PLANE %s\n' "$dpi_out"

printf 'FSCK_RC %s\n' "$fsck_rc"
REMOTE
)"
  printf '%s\n' "$guest_out" >"$run_dir/verify-output.txt"
  qmp_hard_poweroff "$run_dir/verify-qmp.sock"

  local case_json="$VM_DIR/results/cases/${case_id}.json"
  python3 - "$case_id" "$sync_mode" "$scenario" "$run_dir/verify-output.txt" "$oracle_log" "$prev_oracle_log" "$case_json" <<'PY'
import json, pathlib, sys

case_id, sync_mode, scenario, verify_path, oracle_path, prev_oracle_path, out_path = sys.argv[1:8]
lines = pathlib.Path(verify_path).read_text(encoding="utf-8").splitlines()

verify_result = None
doctor_check = None
doctor_dpi = None
fsck_rc = None

for line in lines:
    if line.startswith("VERIFY_RESULT "):
        verify_result = json.loads(line[len("VERIFY_RESULT "):])
    elif line.startswith("DOCTOR_CHECK_INTEGRITY "):
        doctor_check = json.loads(line[len("DOCTOR_CHECK_INTEGRITY "):])
    elif line.startswith("DOCTOR_DATA_PLANE "):
        doctor_dpi = json.loads(line[len("DOCTOR_DATA_PLANE "):])
    elif line.startswith("FSCK_RC "):
        fsck_rc = int(line.split()[1])

if not verify_result or not verify_result.get("ok"):
    raise SystemExit(f"case {case_id}: VERIFY_RESULT failed: {verify_result}")
if scenario == "erasure_lock_fault" and verify_result.get("stale_lock_present_before_open") is not True:
    raise SystemExit(
        f"case {case_id}: erasure_lock_fault expected stale_lock_present_before_open == true, "
        f"got {verify_result.get('stale_lock_present_before_open')!r}"
    )
if not doctor_check:
    raise SystemExit(f"case {case_id}: missing DOCTOR_CHECK_INTEGRITY output")
if not doctor_dpi or doctor_dpi.get("status") != "clean":
    raise SystemExit(f"case {case_id}: DOCTOR_DATA_PLANE not clean: {doctor_dpi}")
dpi_findings = len(doctor_dpi.get("report", {}).get("findings", []))
if dpi_findings != 0:
    raise SystemExit(f"case {case_id}: DOCTOR_DATA_PLANE reported {dpi_findings} findings")

oracle_text = pathlib.Path(oracle_path).read_text(encoding="utf-8", errors="replace")
wal_bytes = None
for line in oracle_text.splitlines():
    if line.startswith("WAL_BYTES "):
        wal_bytes = int(line.split()[1])

record = {
    "case_id": case_id,
    "ok": True,
    "sync_mode": sync_mode,
    "scenario": scenario,
    "fsck_rc": fsck_rc,
    "wal_bytes": wal_bytes,
    "verify_result": verify_result,
    "doctor_check_integrity": doctor_check,
    "doctor_data_plane_integrity_status": doctor_dpi.get("status"),
    "doctor_data_plane_findings": dpi_findings,
    "oracle_log": oracle_path,
    "prev_oracle_log": prev_oracle_path or None,
}
pathlib.Path(out_path).write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
PY
  log "PASS $case_id"
}

run_single_cut_case() {
  local case_id="$1"
  local sync_mode="$2"
  local scenario="$3"
  local fault_mode="$4"
  local allow_drop_rollback="$5"

  log "START $case_id (sync_mode=$sync_mode scenario=$scenario fault_mode=$fault_mode)"
  local run_dir="$VM_DIR/work/$case_id"
  rm -rf "$run_dir"
  mkdir -p "$run_dir"
  ACTIVE_RUN_DIR="$run_dir"

  local data_disk="$run_dir/data.raw"
  local fault_root="$run_dir/fault-root.qcow2"
  local oracle_log="$run_dir/oracle.log"
  local qmp_sock="$run_dir/fault-qmp.sock"

  create_fresh_data_disk "$data_disk"
  create_ephemeral_root "$fault_root"
  start_vm "$fault_root" "$data_disk" "$oracle_log" "$qmp_sock" "$run_dir/fault-console.log"
  wait_for_ssh 120
  guest_setup_flakey_mount "format"

  local extra_run_flags=""
  local marker="READY_FOR_CUT"
  if [ "$fault_mode" = "drop_writes" ] || [ "$fault_mode" = "error_writes" ]; then
    extra_run_flags="--wait-signal-file /tmp/flakey-signal --step-delay-ms 30"
  else
    extra_run_flags="--step-delay-ms 120 --hold-after-ms 1000"
  fi
  if [ "$fault_mode" = "error_writes" ]; then
    marker="READY_FOR_ERROR_WRITES"
  fi

  guest_ssh "nohup /usr/local/bin/fathomdb-dmflakey-harness run \
    --db /mnt/flakey/fathom.db \
    --sync-mode '$sync_mode' \
    --scenario '$scenario' \
    --oracle-device /dev/virtio-ports/oracle.0 \
    $extra_run_flags >/tmp/harness.out 2>/tmp/harness.err </dev/null &"

  wait_for_oracle_marker "$oracle_log" "$marker" 90 || die "$case_id: timed out waiting for $marker in $oracle_log"

  if [ "$fault_mode" = "drop_writes" ]; then
    guest_switch_flakey_mode "drop_writes"
    guest_ssh "touch /tmp/flakey-signal"
    # Allow phase-2 writes/erasure/checkpoint operations to stream into drop_writes, then hard power-off.
    wait_for_oracle_marker "$oracle_log" "SCENARIO_COMPLETE" 4 || true
    sleep 0.25
  elif [ "$fault_mode" = "error_writes" ]; then
    guest_switch_flakey_mode "error_writes"
    guest_ssh "touch /tmp/flakey-signal"
    wait_for_oracle_marker "$oracle_log" "ERROR_WRITES_OBSERVED" 45 || die "$case_id: timed out waiting for ERROR_WRITES_OBSERVED"
  else
    # up_interval hard power-off: cut power genuinely mid-stream BEFORE SCENARIO_COMPLETE.
    sleep 0.20
  fi

  log "CUT_POWER $case_id via QMP quit"
  qmp_hard_poweroff "$qmp_sock"

  verify_after_reboot "$case_id" "$sync_mode" "$scenario" "$data_disk" "$oracle_log" "" "$allow_drop_rollback" "$run_dir"
}

run_reopen_replay_case() {
  local case_id="$1"
  local sync_mode="$2"
  local phase2_fault_mode="$3"

  log "START $case_id (sync_mode=$sync_mode scenario=reopen_replay phase2_fault_mode=$phase2_fault_mode two-cut)"
  local run_dir="$VM_DIR/work/$case_id"
  rm -rf "$run_dir"
  mkdir -p "$run_dir"
  ACTIVE_RUN_DIR="$run_dir"

  local data_disk="$run_dir/data.raw"
  local phase1_root="$run_dir/phase1-root.qcow2"
  local phase2_root="$run_dir/phase2-root.qcow2"
  local oracle1="$run_dir/oracle-phase1.log"
  local oracle2="$run_dir/oracle-phase2.log"

  # Cut 1: seed un-checkpointed WAL accumulation and hard power-off mid-stream.
  create_fresh_data_disk "$data_disk"
  create_ephemeral_root "$phase1_root"
  start_vm "$phase1_root" "$data_disk" "$oracle1" "$run_dir/p1-qmp.sock" "$run_dir/p1-console.log"
  wait_for_ssh 120
  guest_setup_flakey_mount "format"

  guest_ssh "nohup /usr/local/bin/fathomdb-dmflakey-harness run \
    --db /mnt/flakey/fathom.db \
    --sync-mode '$sync_mode' \
    --scenario wal_accum \
    --count 24 \
    --step-delay-ms 120 \
    --hold-after-ms 1000 \
    --oracle-device /dev/virtio-ports/oracle.0 \
    >/tmp/harness-p1.out 2>/tmp/harness-p1.err </dev/null &"

  wait_for_oracle_marker "$oracle1" "READY_FOR_CUT" 90 || die "$case_id: phase1 timed out waiting for READY_FOR_CUT"
  sleep 0.20
  log "CUT_POWER_1 $case_id via QMP quit"
  qmp_hard_poweroff "$run_dir/p1-qmp.sock"

  # Cut 2: reboot cleanly (NO verify yet!), run reopen_replay as FIRST Engine::open to replay dirty WAL + write more, then inject phase-2 fault.
  create_ephemeral_root "$phase2_root"
  start_vm "$phase2_root" "$data_disk" "$oracle2" "$run_dir/p2-qmp.sock" "$run_dir/p2-console.log"
  wait_for_ssh 120
  guest_setup_flakey_mount "reuse"

  local phase2_flags=""
  local allow_drop_rollback="0"
  if [ "$phase2_fault_mode" = "drop_writes" ]; then
    phase2_flags="--wait-signal-file /tmp/flakey-signal --step-delay-ms 60"
    allow_drop_rollback="1"
  elif [ "$phase2_fault_mode" = "poweroff" ]; then
    phase2_flags="--step-delay-ms 100 --hold-after-ms 1000"
    allow_drop_rollback="0"
  else
    die "$case_id: unknown phase2_fault_mode: $phase2_fault_mode"
  fi

  guest_ssh "nohup /usr/local/bin/fathomdb-dmflakey-harness run \
    --db /mnt/flakey/fathom.db \
    --sync-mode '$sync_mode' \
    --scenario reopen_replay \
    --count 24 \
    $phase2_flags \
    --oracle-device /dev/virtio-ports/oracle.0 \
    >/tmp/harness-p2.out 2>/tmp/harness-p2.err </dev/null &"

  wait_for_oracle_marker "$oracle2" "READY_FOR_CUT" 90 || die "$case_id: phase2 timed out waiting for READY_FOR_CUT"
  if [ "$phase2_fault_mode" = "drop_writes" ]; then
    guest_switch_flakey_mode "drop_writes"
    guest_ssh "touch /tmp/flakey-signal"
    wait_for_oracle_marker "$oracle2" "SCENARIO_COMPLETE" 4 || true
    sleep 0.25
  else
    sleep 0.25
  fi
  log "CUT_POWER_2 $case_id via QMP quit"
  qmp_hard_poweroff "$run_dir/p2-qmp.sock"

  verify_after_reboot "$case_id" "$sync_mode" "reopen_replay" "$data_disk" "$oracle2" "$oracle1" "$allow_drop_rollback" "$run_dir"
}

run_case() {
  local case_id="$1"
  case "$case_id" in
    normal_active_commit_poweroff)
      run_single_cut_case "$case_id" "normal" "active_commit" "poweroff" "0"
      ;;
    normal_wal_accum_drop_writes)
      run_single_cut_case "$case_id" "normal" "wal_accum" "drop_writes" "1"
      ;;
    normal_checkpoint_erasure_drop_writes)
      run_single_cut_case "$case_id" "normal" "checkpoint_erasure" "drop_writes" "1"
      ;;
    normal_reopen_replay_double_cut)
      run_reopen_replay_case "$case_id" "normal" "drop_writes"
      ;;
    normal_erasure_lock_fault)
      run_single_cut_case "$case_id" "normal" "erasure_lock_fault" "poweroff" "0"
      ;;
    full_active_commit_poweroff)
      run_single_cut_case "$case_id" "full" "active_commit" "poweroff" "0"
      ;;
    full_wal_accum_poweroff)
      run_single_cut_case "$case_id" "full" "wal_accum" "poweroff" "0"
      ;;
    full_checkpoint_erasure_poweroff)
      run_single_cut_case "$case_id" "full" "checkpoint_erasure" "poweroff" "0"
      ;;
    full_reopen_replay_double_cut)
      run_reopen_replay_case "$case_id" "full" "poweroff"
      ;;
    full_erasure_lock_fault)
      run_single_cut_case "$case_id" "full" "erasure_lock_fault" "poweroff" "0"
      ;;
    full_error_writes_eio)
      run_single_cut_case "$case_id" "full" "error_writes_full" "error_writes" "0"
      ;;
    *)
      die "unhandled case_id: $case_id"
      ;;
  esac
}

write_campaign_summary() {
  local summary_path="$VM_DIR/results/campaign-summary.json"
  python3 - "$VM_DIR/results/cases" "$summary_path" "${SELECTED_CASES[@]}" <<'PY'
import datetime, json, pathlib, sys

cases_dir = pathlib.Path(sys.argv[1])
summary_path = pathlib.Path(sys.argv[2])
case_ids = sys.argv[3:]

cases = []
all_ok = True
for cid in case_ids:
    p = cases_dir / f"{cid}.json"
    if not p.exists():
        all_ok = False
        cases.append({"case_id": cid, "ok": False, "error": "missing case receipt"})
        continue
    data = json.loads(p.read_text(encoding="utf-8"))
    if not data.get("ok"):
        all_ok = False
    cases.append(data)

summary = {
    "schema_version": "fathomdb.dmflakey.campaign.v1",
    "generated_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "total_cases": len(cases),
    "passed_cases": sum(1 for c in cases if c.get("ok")),
    "all_ok": all_ok,
    "cases": cases,
}
summary_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
print(f"CAMPAIGN_SUMMARY path={summary_path} passed={summary['passed_cases']}/{summary['total_cases']} all_ok={all_ok}")
if not all_ok:
    sys.exit(1)
PY
}

main() {
  parse_args "$@"
  if [ "$SELF_TEST" -eq 1 ]; then
    run_self_test
    exit 0
  fi
  if [ "$DRY_RUN" -eq 1 ]; then
    run_dry_run
    exit 0
  fi

  require_host_tools
  select_cases
  [ -x "$HARNESS_BIN" ] || die "harness binary missing or not executable: $HARNESS_BIN"
  [ -x "$CLI_BIN" ] || die "fathomdb CLI binary missing or not executable: $CLI_BIN"

  verify_cloud_image
  prepare_ssh_and_cidata
  pick_free_ssh_port
  prepare_guest_template

  local case_id
  for case_id in "${SELECTED_CASES[@]}"; do
    run_case "$case_id"
  done

  write_campaign_summary
  log "ALL DM-FLAKEY CAMPAIGN CASES PASSED (active_run_dir=$ACTIVE_RUN_DIR)"
}

main "$@"
