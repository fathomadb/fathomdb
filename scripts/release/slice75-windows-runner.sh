#!/usr/bin/env bash
# Start and fail-closed verify the dedicated Windows release runner.
set -euo pipefail

usage() {
  printf 'usage: %s --vm NAME --repository OWNER/REPO --runner NAME --ssh-user USER --ssh-key FILE --receipt FILE\n' "$0" >&2
}

vm='' repository='' runner='' ssh_user='' ssh_key='' receipt=''
while [ "$#" -gt 0 ]; do
  [ "$#" -ge 2 ] || { usage; exit 2; }
  case "$1" in
    --vm) vm="$2" ;;
    --repository) repository="$2" ;;
    --runner) runner="$2" ;;
    --ssh-user) ssh_user="$2" ;;
    --ssh-key) ssh_key="$2" ;;
    --receipt) receipt="$2" ;;
    *) usage; exit 2 ;;
  esac
  shift 2
done
[ -n "$vm" ] && [ -n "$repository" ] && [ -n "$runner" ] && [ -n "$ssh_user" ] \
  && [ -f "$ssh_key" ] && [ -n "$receipt" ] || { usage; exit 2; }
[[ "$repository" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]] \
  && [[ "$runner" =~ ^[A-Za-z0-9_.-]+$ ]] \
  || { printf 'slice75-windows-runner: unsafe repository or runner name\n' >&2; exit 2; }
for command in virsh ssh gh python3; do
  command -v "$command" >/dev/null || { printf 'slice75-windows-runner: missing %s\n' "$command" >&2; exit 1; }
done
gh auth status >/dev/null 2>&1 || { printf 'slice75-windows-runner: gh authentication is unavailable\n' >&2; exit 1; }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

state="$(virsh domstate "$vm" | tr -d '\r')"
if [ "$state" != running ]; then
  virsh start "$vm" >/dev/null
fi
ip=''
for _ in $(seq 1 60); do
  ip="$(virsh domifaddr "$vm" --source lease 2>/dev/null \
    | awk '/ipv4/ {sub("/.*", "", $4); print $4; exit}')"
  [ -z "$ip" ] || break
  sleep 2
done
[ -n "$ip" ] || { printf 'slice75-windows-runner: VM has no leased IPv4 address\n' >&2; exit 1; }

ssh_args=(-o BatchMode=yes -o StrictHostKeyChecking=accept-new -i "$ssh_key" "$ssh_user@$ip")
metadata="$(ssh "${ssh_args[@]}" \
  "powershell -NoProfile -NonInteractive -Command \"Get-Content -Raw C:\\actions-runner\\.runner\"")"
python3 - "$metadata" "$repository" "$runner" <<'PY'
import json, sys
value = json.loads(sys.argv[1])
expected_url = "https://github.com/" + sys.argv[2]
if value.get("gitHubUrl", "").rstrip("/") != expected_url or value.get("agentName") != sys.argv[3]:
    raise SystemExit(
        "slice75-windows-runner: runner registration differs; "
        f"found {value.get('gitHubUrl')} / {value.get('agentName')}"
    )
PY
service_name="actions.runner.${repository//\//-}.$runner"
# The validated service name must be expanded into the remote command.
# shellcheck disable=SC2029
ssh "${ssh_args[@]}" \
  "powershell -NoProfile -NonInteractive -Command \"Start-Service -Name '$service_name'; (Get-Service -Name '$service_name').Status.ToString()\"" \
  | tr -d '\r' | grep -Fx Running >/dev/null

gh api --paginate "repos/$repository/actions/runners?per_page=100" > "$work/runners.json"
python3 - "$work/runners.json" "$repository" "$runner" "$vm" "$ip" "$service_name" "$receipt" <<'PY'
import json, pathlib, sys
source, repository, runner_name, vm, ip, service, receipt = sys.argv[1:]
value = json.loads(pathlib.Path(source).read_text())
matches = [runner for runner in value["runners"] if runner["name"] == runner_name]
if len(matches) != 1:
    raise SystemExit(f"slice75-windows-runner: expected one repository runner, found {len(matches)}")
runner = matches[0]
labels = sorted(label["name"] for label in runner["labels"])
expected = sorted(["self-hosted", "Windows", "X64", "windchill3-windows-11"])
if labels != expected or runner["status"] != "online" or runner["busy"]:
    raise SystemExit(
        f"slice75-windows-runner: runner not idle/online with exact labels: "
        f"status={runner['status']} busy={runner['busy']} labels={labels}"
    )
payload = {
    "schema_version": "fathomdb.slice75-windows-runner/v1",
    "repository": repository,
    "runner": runner_name,
    "vm": vm,
    "ip": ip,
    "service_name": service,
    "service": "running",
    "status": "online",
    "busy": False,
    "labels": ["self-hosted", "Windows", "X64", "windchill3-windows-11"],
}
path = pathlib.Path(receipt)
path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True) + "\n")
PY
printf 'slice75-windows-runner: pass; receipt at %s\n' "$receipt"
