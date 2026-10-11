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
#   destroy --name fdb-lab-<x> --zone <zone>   (only deletes labeled lab VMs)
#   status                                     (lists lab VMs in the project)
#   wait-ssh --name <n> --zone <z>             (polls until SSH answers)
#   run     --name <n> --zone <z> -- <remote command...>
#   push    --name <n> --zone <z> <local-path> <remote-path>
#   pull    --name <n> --zone <z> <remote-path> <local-dir>
#   pilot   --profile cpu|t4 --zone <z> [--ref <branch>] [--out <dir>]
#           [--max-run 2h] [--keep]
#           create → wait-ssh → remote build + receipt → pull → destroy.
#           The VM is destroyed on any exit unless --keep is given.
#
# The project is never taken from the active gcloud configuration: set
# FDB_LAB_PROJECT (or pass --project) so a run cannot land in the wrong one.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LAB_LABEL="fathomdb-lab"
NAME_RE='^fdb-lab-[a-z0-9-]{1,40}$'
MAX_RUN_CAP_MINUTES=240

die() { printf 'gcp-cuda-lab: %s\n' "$*" >&2; exit 2; }
note() { printf 'gcp-cuda-lab: %s\n' "$*" >&2; }

usage() { sed -n '/^# Usage:/,/^# The project/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2; }

PROJECT="${FDB_LAB_PROJECT:-}"
PROFILE="" NAME="" ZONE="" MAX_RUN="2h" REF="release/0.8.28" OUT="" KEEP=0
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
      PROFILE_FLAGS=(--machine-type=e2-standard-8 --boot-disk-size=60GB) ;;
    t4)
      # GPUs cannot live-migrate, so host maintenance must terminate the VM.
      PROFILE_FLAGS=(--machine-type=n1-standard-4 --boot-disk-size=100GB
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
  local attempt
  for attempt in $(seq 1 30); do
    if gcloud compute ssh "$NAME" "${SSH_FLAGS[@]}" --command=true >/dev/null 2>&1; then
      return 0
    fi
    note "waiting for SSH on $NAME (attempt $attempt/30)"
    sleep 10
  done
  die "SSH to $NAME never came up"
}

cmd_run() {
  require_project; require_zone; require_lab_name; ssh_target_flags
  [ "${#POSITIONAL[@]}" -gt 0 ] || die "run needs a remote command after --"
  gcloud compute ssh "$NAME" "${SSH_FLAGS[@]}" --command="${POSITIONAL[*]}"
}

cmd_push() {
  require_project; require_zone; require_lab_name; ssh_target_flags
  [ "${#POSITIONAL[@]}" -eq 2 ] || die "push needs <local-path> <remote-path>"
  gcloud compute scp "${SSH_FLAGS[@]}" --recurse "${POSITIONAL[0]}" "$NAME:${POSITIONAL[1]}"
}

cmd_pull() {
  require_project; require_zone; require_lab_name; ssh_target_flags
  [ "${#POSITIONAL[@]}" -eq 2 ] || die "pull needs <remote-path> <local-dir>"
  mkdir -p "${POSITIONAL[1]}"
  gcloud compute scp "${SSH_FLAGS[@]}" --recurse "$NAME:${POSITIONAL[0]}" "${POSITIONAL[1]}"
}

cmd_pilot() {
  require_project; require_zone
  [ -n "$PROFILE" ] || die "--profile is required"
  local run_id
  run_id="$(date -u +%Y%m%d-%H%M%S)"
  NAME="fdb-lab-$PROFILE-$run_id"
  [ -n "$OUT" ] || OUT="$ROOT/target/gcp-lab/$NAME"
  require_lab_name; require_max_run; profile_flags

  cmd_create
  if [ "$KEEP" -eq 0 ]; then
    trap 'note "tearing down $NAME"; cmd_destroy || note "TEARDOWN FAILED: delete $NAME by hand"' EXIT
  fi
  cmd_wait_ssh

  POSITIONAL=("$ROOT/scripts/gcp-lab/remote-build.sh" "fdb-lab-remote-build.sh")
  cmd_push
  local status=0
  POSITIONAL=(bash fdb-lab-remote-build.sh --ref "$REF" --profile "$PROFILE" --run-id "$NAME")
  cmd_run || status=$?
  POSITIONAL=("fdb-lab-receipts" "$OUT")
  cmd_pull || note "could not pull receipts from $NAME"
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
    pilot) cmd_pilot ;;
    -h|--help|help) usage ;;
    *) usage; die "unknown command: $command" ;;
  esac
}

main "$@"
