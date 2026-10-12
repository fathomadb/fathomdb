#!/usr/bin/env bash
# Guardrail tests for scripts/gcp-cuda-lab.sh. A stub `gcloud` on PATH records
# every invocation, so no cloud call is ever made.
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel)"
LAB="$ROOT/scripts/gcp-cuda-lab.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

mkdir -p "$TMP/bin"
cat >"$TMP/bin/gcloud" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$STUB_LOG"
case "$*" in
  *"compute instances describe"*) printf '%s\n' "${STUB_LABEL:-}" ;;
  *"compute scp"*"remote-build.sh"*)
    if [ -n "${STUB_SCP_FAIL_ALWAYS:-}" ]; then exit 255; fi
    if [ -n "${STUB_SCP_RESET_ONCE:-}" ] && [ ! -e "$STUB_SCP_RESET_ONCE" ]; then
      : >"$STUB_SCP_RESET_ONCE"; exit 255
    fi ;;
  *"bash fdb-lab-remote-build.sh"*)
    # Simulate one SSH connection reset (exit 255) before the build starts.
    if [ -n "${STUB_SSH_RESET_ONCE:-}" ] && [ ! -e "$STUB_SSH_RESET_ONCE" ]; then
      : >"$STUB_SSH_RESET_ONCE"; exit 255
    fi ;;
esac
exit 0
STUB
chmod +x "$TMP/bin/gcloud"
export PATH="$TMP/bin:$PATH" STUB_LOG="$TMP/gcloud.log"

fail() { echo "FAIL $*"; exit 1; }
reset_log() { : >"$STUB_LOG"; }
expect_in_log() {
  if grep -qF -- "$1" "$STUB_LOG"; then return 0; fi
  echo "gcloud calls:"; cat "$STUB_LOG"
  fail "$2: missing '$1'"
}
expect_not_in_log() { if grep -qF -- "$1" "$STUB_LOG"; then fail "$2: unexpected '$1'"; fi; }
expect_refused() {
  local what="$1"; shift
  reset_log
  if "$@" >/dev/null 2>&1; then fail "$what was accepted"; fi
  expect_not_in_log "instances create" "$what"
  expect_not_in_log "instances delete" "$what"
}

export FDB_LAB_PROJECT=proj-test
export FDB_LAB_SSH_RETRY_SLEEP=0 FDB_LAB_SSH_SETTLE_SLEEP=0

# CPU profile: Spot, capped, self-deleting, no GPU, no service account.
reset_log
"$LAB" create --profile cpu --name fdb-lab-cpu1 --zone us-central1-a >/dev/null
for flag in "compute instances create fdb-lab-cpu1" --project=proj-test --zone=us-central1-a \
  --provisioning-model=SPOT --max-run-duration=2h --instance-termination-action=DELETE \
  --labels=fathomdb-lab=true --no-service-account --no-scopes \
  --metadata=block-project-ssh-keys=TRUE --quiet; do
  expect_in_log "$flag" "cpu create"
done
expect_not_in_log --accelerator "cpu create"

# T4 profile: one T4, host maintenance terminates (GPUs cannot live-migrate).
reset_log
"$LAB" create --profile t4 --name fdb-lab-t4a --zone us-central1-b --max-run 3h >/dev/null
for flag in --accelerator=count=1,type=nvidia-tesla-t4 --maintenance-policy=TERMINATE \
  --machine-type=n1-standard-4 --max-run-duration=3h --instance-termination-action=DELETE; do
  expect_in_log "$flag" "t4 create"
done

# Refusals: no project, foreign names, unbounded run time, unknown profile.
expect_refused "missing project" env -u FDB_LAB_PROJECT "$LAB" create --profile cpu --name fdb-lab-x --zone us-central1-a
expect_refused "foreign create name" "$LAB" create --profile cpu --name instance-1 --zone us-central1-a
expect_refused "run time over 4h" "$LAB" create --profile cpu --name fdb-lab-x --zone us-central1-a --max-run 5h
expect_refused "unknown profile" "$LAB" create --profile a100 --name fdb-lab-x --zone us-central1-a
expect_refused "foreign destroy name" "$LAB" destroy --name instance-1 --zone us-central1-a

# Destroy deletes only instances that carry the lab label.
STUB_LABEL="" expect_refused "unlabeled destroy" "$LAB" destroy --name fdb-lab-cpu1 --zone us-central1-a
reset_log
STUB_LABEL=true "$LAB" destroy --name fdb-lab-cpu1 --zone us-central1-a >/dev/null
expect_in_log "compute instances delete fdb-lab-cpu1" "labeled destroy"
expect_in_log --delete-disks=all "labeled destroy"
expect_in_log --project=proj-test "labeled destroy"

# A T4 pilot installs the pinned driver branch before building, and tears down.
reset_log
STUB_LABEL=true "$LAB" pilot --profile t4 --zone us-central1-b --driver 580 \
  --out "$TMP/out" >/dev/null 2>&1
expect_in_log "remote-driver.sh" "t4 pilot pushes the driver script"
expect_in_log "--driver-branch 580" "t4 pilot pins the driver branch"
expect_in_log "fdb-lab-remote-build.sh" "t4 pilot still builds"
expect_in_log "compute instances delete fdb-lab-t4-" "t4 pilot tears down"
driver_line="$(grep -n -m1 -- '--driver-branch' "$STUB_LOG")"
build_line="$(grep -n -m1 -- 'bash fdb-lab-remote-build.sh' "$STUB_LOG")"
driver_line="${driver_line%%:*}" build_line="${build_line%%:*}"
[ "$driver_line" -lt "$build_line" ] || fail "driver must install before the build"

# A GPU pilot without a driver branch is refused before any VM exists.
expect_refused "t4 pilot without driver" "$LAB" pilot --profile t4 --zone us-central1-b
expect_refused "non-numeric driver" "$LAB" pilot --profile t4 --zone us-central1-b --driver latest

# A CPU pilot installs no driver.
reset_log
STUB_LABEL=true "$LAB" pilot --profile cpu --zone us-central1-a --out "$TMP/out" >/dev/null 2>&1
expect_not_in_log "remote-driver.sh" "cpu pilot"

# A connection reset (ssh exit 255) before a remote step is retried.
reset_log
STUB_LABEL=true STUB_SSH_RESET_ONCE="$TMP/reset-once" FDB_LAB_SSH_RETRY_SLEEP=0 \
  "$LAB" pilot --profile cpu --zone us-central1-a --out "$TMP/out" >/dev/null 2>&1 \
  || fail "pilot did not recover from one ssh connection reset"
build_calls="$(grep -c -- 'bash fdb-lab-remote-build.sh' "$STUB_LOG")"
[ "$build_calls" -eq 2 ] || fail "expected one retry of the build step, saw $build_calls calls"

# A connection reset during a copy is retried too.
reset_log
STUB_LABEL=true STUB_SCP_RESET_ONCE="$TMP/scp-once" FDB_LAB_SSH_RETRY_SLEEP=0 \
  "$LAB" pilot --profile cpu --zone us-central1-a --out "$TMP/out" >/dev/null 2>&1 \
  || fail "pilot did not recover from one scp connection reset"

# When a step fails for good, receipts are still pulled before teardown.
reset_log
if STUB_LABEL=true STUB_SCP_FAIL_ALWAYS=1 FDB_LAB_SSH_RETRY_SLEEP=0 \
  "$LAB" pilot --profile cpu --zone us-central1-a --out "$TMP/out" >/dev/null 2>&1; then
  fail "pilot passed although the build script could not be copied"
fi
pull_line="$(grep -n -m1 -- 'fdb-lab-receipts' "$STUB_LOG")"
delete_line="$(grep -n -m1 -- 'compute instances delete' "$STUB_LOG")"
pull_line="${pull_line%%:*}" delete_line="${delete_line%%:*}"
[ -n "$pull_line" ] && [ -n "$delete_line" ] && [ "$pull_line" -lt "$delete_line" ] \
  || fail "a failed pilot must pull receipts before teardown"

# Status lists only lab instances, with data reduction.
reset_log
"$LAB" status >/dev/null
expect_in_log "compute instances list" "status"
expect_in_log --filter=labels.fathomdb-lab=true "status"

echo 'All gcp-cuda-lab guardrail tests passed'
