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

# Status lists only lab instances, with data reduction.
reset_log
"$LAB" status >/dev/null
expect_in_log "compute instances list" "status"
expect_in_log --filter=labels.fathomdb-lab=true "status"

echo 'All gcp-cuda-lab guardrail tests passed'
