#!/usr/bin/env bash
# scripts/gcp-cuda-lab.sh — short-lived Compute Engine VMs for the Slice 31
# CUDA driver/library compatibility cells (dev/plans/0.8.28/features/slice-31/
# research/cloud-gpu-test-options.md).
#
# Every VM this script creates is Spot, carries the label fathomdb-lab=true,
# has a hard run-time cap (default 2h, max 4h) and deletes itself when the cap
# or a preemption hits, so a forgotten VM cannot keep billing. The VM gets no
# service account: it never needs Google Cloud API access. SSH keys go into
# instance metadata, never the project-wide ssh-keys.
#
# Usage:
#   FDB_LAB_PROJECT=<project> scripts/gcp-cuda-lab.sh <command> [options]
#
#   create  --profile cpu|t4 --name fdb-lab-<x> --zone <zone> [--max-run 2h]
#           [--machine-type n1-<...>]   (t4 only; default n1-standard-4)
#   destroy --name fdb-lab-<x> --zone <zone>   (only deletes labeled lab VMs)
#   status                                     (lists lab VMs in the project)
#   wait-ssh --name <n> --zone <z>             (polls until SSH answers)
#   run     --name <n> --zone <z> -- <remote command...>
#   push    --name <n> --zone <z> <local-path> <remote-path>
#   pull    --name <n> --zone <z> <remote-path> <local-dir>
#   driver  --name <n> --zone <z> --driver <branch> [--driver-open]
#           [--kernel <release>]
#           (installs a pinned NVIDIA branch on an existing lab VM, rebooting
#           once if the module needs it; --driver-open takes the open kernel
#           modules, --kernel boots that GCP kernel, e.g. 7.0.0-1014-gcp,
#           with the driver built for it, sharing the one reboot)
#   pilot   --profile cpu|t4 --zone <z> [--driver <branch>] [--driver-open]
#           [--kernel <release>] [--ref <branch>] [--out <dir>]
#           [--max-run 2h] [--keep]
#           create → wait-ssh → [driver] → remote build + receipt → pull →
#           destroy. The t4 profile needs --driver (an NVIDIA branch such as
#           580). The VM is destroyed on any exit unless --keep is given.
#
# The project is never taken from the active gcloud configuration: set
# FDB_LAB_PROJECT (or pass --project) so a run cannot land in the wrong one.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LAB_LABEL="fathomdb-lab"
NAME_RE='^fdb-lab-[a-z0-9-]{1,40}$'
KERNEL_RE='^[0-9]+\.[0-9]+\.[0-9]+-[0-9]+-gcp$'
MAX_RUN_CAP_MINUTES=240

die() { printf 'gcp-cuda-lab: %s\n' "$*" >&2; exit 2; }
note() { printf 'gcp-cuda-lab: %s\n' "$*" >&2; }

usage() { sed -n '/^# Usage:/,/^# The project/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2; }

PROJECT="${FDB_LAB_PROJECT:-}"
PROFILE="" NAME="" ZONE="" MAX_RUN="2h" REF="release/0.8.28" OUT="" KEEP=0 DRIVER="" MACHINE_TYPE=""
DRIVER_OPEN=0 KERNEL=""
POSITIONAL=()

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      --project) PROJECT="${2:?--project needs a value}"; shift 2 ;;
      --profile) PROFILE="${2:?--profile needs a value}"; shift 2 ;;
      --name) NAME="${2:?--name needs a value}"; shift 2 ;;
      --zone) ZONE="${2:?--zone needs a value}"; shift 2 ;;
      --max-run) MAX_RUN="${2:?--max-run needs a value}"; shift 2 ;;
      --ref) REF="${2:?--ref needs a value}"; shift 2 ;;
      --out) OUT="${2:?--out needs a value}"; shift 2 ;;
      --driver) DRIVER="${2:?--driver needs a value}"; shift 2 ;;
      --machine-type) MACHINE_TYPE="${2:?--machine-type needs a value}"; shift 2 ;;
      --driver-open) DRIVER_OPEN=1; shift ;;
      --kernel) KERNEL="${2:?--kernel needs a value}"; shift 2 ;;
      --keep) KEEP=1; shift ;;
      --) shift; POSITIONAL+=("$@"); break ;;
      -*) die "unknown option: $1" ;;
      *) POSITIONAL+=("$1"); shift ;;
    esac
  done
}

require_project() { [ -n "$PROJECT" ] || die "set FDB_LAB_PROJECT or pass --project"; }
require_zone() { [ -n "$ZONE" ] || die "--zone is required"; }
require_lab_name() {
  [ -n "$NAME" ] || die "--name is required"
  [[ "$NAME" =~ $NAME_RE ]] || die "refusing name '$NAME': lab VMs must match $NAME_RE"
}

duration_minutes() {
  local d="$1" total=0 n unit
  [[ "$d" =~ ^([0-9]+[hm])+$ ]] || return 1
  while [[ "$d" =~ ^([0-9]+)([hm])(.*)$ ]]; do
    n="${BASH_REMATCH[1]}" unit="${BASH_REMATCH[2]}" d="${BASH_REMATCH[3]}"
    if [ "$unit" = h ]; then total=$((total + n * 60)); else total=$((total + n)); fi
  done
  printf '%s\n' "$total"
}

require_max_run() {
  local minutes
  minutes="$(duration_minutes "$MAX_RUN")" || die "--max-run must look like 90m or 2h (got '$MAX_RUN')"
  [ "$minutes" -ge 10 ] || die "--max-run below 10m leaves no time to work"
  [ "$minutes" -le "$MAX_RUN_CAP_MINUTES" ] || die "--max-run $MAX_RUN exceeds the 4h cap"
}

profile_flags() {
  case "$PROFILE" in
    cpu)
      [ -z "$MACHINE_TYPE" ] || die "the cpu profile has a fixed machine type; drop --machine-type"
      PROFILE_FLAGS=(--machine-type=e2-standard-8 --boot-disk-size=60GB) ;;
    t4)
      # Only the N1 family can carry a T4.
      local machine="${MACHINE_TYPE:-n1-standard-4}"
      [[ "$machine" =~ ^n1-(standard|highmem|highcpu)-[0-9]+$ ]] \
        || die "the t4 profile needs an n1 machine type (got '$machine')"
      # GPUs cannot live-migrate, so host maintenance must terminate the VM.
      PROFILE_FLAGS=(--machine-type="$machine" --boot-disk-size=100GB
        "--accelerator=count=1,type=nvidia-tesla-t4" --maintenance-policy=TERMINATE) ;;
    *) die "unknown profile '$PROFILE' (expected cpu or t4)" ;;
  esac
}

cmd_create() {
  require_project; require_zone; require_lab_name; require_max_run; profile_flags
  gcloud compute instances create "$NAME" \
    --project="$PROJECT" --zone="$ZONE" \
    "${PROFILE_FLAGS[@]}" \
    --boot-disk-type=pd-balanced \
    --image-family=ubuntu-2404-lts-amd64 --image-project=ubuntu-os-cloud \
    --provisioning-model=SPOT \
    --max-run-duration="$MAX_RUN" --instance-termination-action=DELETE \
    --labels="$LAB_LABEL=true,$LAB_LABEL-profile=$PROFILE" \
    --no-service-account --no-scopes \
    --metadata=block-project-ssh-keys=TRUE \
    --format="value(name,zone.basename(),status)" \
    --quiet
}

cmd_destroy() {
  require_project; require_zone; require_lab_name
  local label
  label="$(gcloud compute instances describe "$NAME" --project="$PROJECT" --zone="$ZONE" \
    --format="value(labels.$LAB_LABEL)" --quiet)"
  [ "$label" = true ] || die "refusing to delete '$NAME': it lacks the $LAB_LABEL=true label"
  gcloud compute instances delete "$NAME" --project="$PROJECT" --zone="$ZONE" \
    --delete-disks=all --quiet
}

cmd_status() {
  require_project
  gcloud compute instances list --project="$PROJECT" \
    --filter="labels.$LAB_LABEL=true" \
    --format="table(name,zone.basename(),status,machineType.basename(),scheduling.terminationTime)" \
    --limit=50 --quiet
}

ssh_target_flags() { SSH_FLAGS=(--project="$PROJECT" --zone="$ZONE" --quiet); }

cmd_wait_ssh() {
  require_project; require_zone; require_lab_name; ssh_target_flags
  # sshd can answer once and then reset connections while first-boot or
  # post-reboot services restart it, so require three answers in a row.
  local attempt streak=0
  for attempt in $(seq 1 40); do
    if gcloud compute ssh "$NAME" "${SSH_FLAGS[@]}" --command=true >/dev/null 2>&1; then
      streak=$((streak + 1))
      [ "$streak" -lt 3 ] || return 0
      sleep "${FDB_LAB_SSH_SETTLE_SLEEP:-5}"
      continue
    fi
    streak=0
    note "waiting for SSH on $NAME (attempt $attempt/40)"
    sleep 10
  done
  die "SSH to $NAME never came up"
}

# ssh and scp exit 255 only for their own failures (connection refused or
# reset); a remote command's own exit status passes through. Retrying 255 is
# safe for copies, and for commands that never started.
ssh_retry() {
  local attempt status
  for attempt in 1 2 3; do
    status=0
    "$@" || status=$?
    [ "$status" -eq 255 ] || return "$status"
    note "connection to $NAME failed (attempt $attempt/3); retrying"
    sleep "${FDB_LAB_SSH_RETRY_SLEEP:-15}"
  done
  return 255
}

cmd_run() {
  require_project; require_zone; require_lab_name; ssh_target_flags
  [ "${#POSITIONAL[@]}" -gt 0 ] || die "run needs a remote command after --"
  ssh_retry gcloud compute ssh "$NAME" "${SSH_FLAGS[@]}" --command="${POSITIONAL[*]}"
}

cmd_push() {
  require_project; require_zone; require_lab_name; ssh_target_flags
  [ "${#POSITIONAL[@]}" -eq 2 ] || die "push needs <local-path> <remote-path>"
  ssh_retry gcloud compute scp "${SSH_FLAGS[@]}" --recurse "${POSITIONAL[0]}" "$NAME:${POSITIONAL[1]}"
}

cmd_pull() {
  require_project; require_zone; require_lab_name; ssh_target_flags
  [ "${#POSITIONAL[@]}" -eq 2 ] || die "pull needs <remote-path> <local-dir>"
  mkdir -p "${POSITIONAL[1]}"
  ssh_retry gcloud compute scp "${SSH_FLAGS[@]}" --recurse "$NAME:${POSITIONAL[0]}" "${POSITIONAL[1]}"
}

require_driver() {
  if [ "$PROFILE" = cpu ]; then
    [ -z "$DRIVER" ] && [ "$DRIVER_OPEN" -eq 0 ] && [ -z "$KERNEL" ] \
      || die "the cpu profile has no GPU; drop --driver, --driver-open and --kernel"
    return 0
  fi
  [ -n "$DRIVER" ] || die "--driver <branch> is required for the $PROFILE profile (e.g. 580)"
  [[ "$DRIVER" =~ ^[0-9]{3}$ ]] || die "--driver must be an NVIDIA branch number such as 580 (got '$DRIVER')"
  [ -z "$KERNEL" ] || [[ "$KERNEL" =~ $KERNEL_RE ]] \
    || die "--kernel must be a GCP kernel release such as 7.0.0-1014-gcp (got '$KERNEL')"
  DRIVER_ARGS=(--driver-branch "$DRIVER")
  [ "$DRIVER_OPEN" -eq 0 ] || DRIVER_ARGS+=(--open)
  [ -z "$KERNEL" ] || DRIVER_ARGS+=(--kernel "$KERNEL")
}

# The driver script exits 100 when the new kernel module, or a new kernel,
# needs a reboot.
install_driver() {
  local status=0
  POSITIONAL=("$ROOT/scripts/gcp-lab/remote-driver.sh" "fdb-lab-remote-driver.sh")
  cmd_push
  POSITIONAL=(bash fdb-lab-remote-driver.sh "${DRIVER_ARGS[@]}")
  cmd_run || status=$?
  if [ "$status" -eq 100 ]; then
    note "rebooting $NAME to load the NVIDIA module"
    # Not cmd_run: the reboot drops the connection (255), and a retry would
    # reboot the VM again.
    gcloud compute ssh "$NAME" "${SSH_FLAGS[@]}" --command="sudo systemctl reboot" || true
    sleep "${FDB_LAB_REBOOT_SLEEP:-20}"
    cmd_wait_ssh
    status=0
    POSITIONAL=(bash fdb-lab-remote-driver.sh "${DRIVER_ARGS[@]}" --verify-only)
    cmd_run || status=$?
  fi
  [ "$status" -eq 0 ] || note "driver $DRIVER install failed on $NAME (exit $status)"
  return "$status"
}

cmd_driver() {
  require_project; require_zone; require_lab_name; ssh_target_flags
  PROFILE=t4 require_driver
  install_driver
}

RECEIPTS_PULLED=0
pull_receipts() {
  [ "$RECEIPTS_PULLED" -eq 0 ] || return 0
  RECEIPTS_PULLED=1
  POSITIONAL=("fdb-lab-receipts" "$OUT")
  cmd_pull || note "could not pull receipts from $NAME"
}

teardown() {
  pull_receipts
  note "tearing down $NAME"
  cmd_destroy || note "TEARDOWN FAILED: delete $NAME by hand"
}

cmd_pilot() {
  require_project; require_zone
  [ -n "$PROFILE" ] || die "--profile is required"
  local run_id
  run_id="$(date -u +%Y%m%d-%H%M%S)"
  NAME="fdb-lab-$PROFILE-$run_id"
  [ -n "$OUT" ] || OUT="$ROOT/target/gcp-lab/$NAME"
  require_lab_name; require_max_run; profile_flags; require_driver

  cmd_create
  if [ "$KEEP" -eq 0 ]; then
    trap teardown EXIT
  fi
  cmd_wait_ssh
  if [ -n "$DRIVER" ] && ! install_driver; then
    die "stopping before the build; receipts go to $OUT/fdb-lab-receipts"
  fi

  POSITIONAL=("$ROOT/scripts/gcp-lab/remote-build.sh" "fdb-lab-remote-build.sh")
  cmd_push
  local status=0
  POSITIONAL=(bash fdb-lab-remote-build.sh --ref "$REF" --profile "$PROFILE" --run-id "$NAME")
  cmd_run || status=$?
  pull_receipts
  note "receipts: $OUT/fdb-lab-receipts (remote build exit $status)"
  return "$status"
}

main() {
  [ "$#" -gt 0 ] || { usage; exit 2; }
  local command="$1"; shift
  parse_args "$@"
  case "$command" in
    create) cmd_create ;;
    destroy) cmd_destroy ;;
    status) cmd_status ;;
    wait-ssh) cmd_wait_ssh ;;
    run) cmd_run ;;
    push) cmd_push ;;
    pull) cmd_pull ;;
    driver) cmd_driver ;;
    pilot) cmd_pilot ;;
    -h|--help|help) usage ;;
    *) usage; die "unknown command: $command" ;;
  esac
}

main "$@"
