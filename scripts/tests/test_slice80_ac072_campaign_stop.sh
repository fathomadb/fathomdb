#!/usr/bin/env bash
# A failed cell verdict must prevent dispatching a later AC-072 observation.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
campaign="$root/scripts/perf-experiments/run-slice80-ac072-campaign.sh"
test -x "$campaign"

temp_dir=$(mktemp -d /tmp/fathomdb-slice80-dispatch.XXXXXX)
cleanup() { rm -rf "$temp_dir"; }
trap cleanup EXIT
cat >"$temp_dir/runner" <<'EOF'
#!/usr/bin/env bash
basename "$3" >>"$DISPATCH_MARKER"
printf 'partial receipt\n' >"$3"
EOF
cat >"$temp_dir/validator" <<'EOF'
import os
from pathlib import Path

Path(os.environ["VALIDATOR_MARKER"]).write_text("invoked\n")
raise SystemExit(1)
EOF
chmod +x "$temp_dir/runner"
chmod 600 "$temp_dir/validator"
runner_sha=$(sha256sum "$temp_dir/runner" | awk '{print $1}')
scanner_sha=$(sha256sum "$root/dev/tools/slice80_read_acceptance.py" | awk '{print $1}')
validator_sha=$(sha256sum "$temp_dir/validator" | awk '{print $1}')
dispatcher_sha=$(sha256sum "$campaign" | awk '{print $1}')
if DISPATCH_MARKER="$temp_dir/labels" VALIDATOR_MARKER="$temp_dir/validator-marker" SLICE80_AC072_RUNNER="$temp_dir/runner" \
  SLICE80_AC072_VALIDATOR="$temp_dir/validator" "$campaign" "$root" /bin/true \
  "$temp_dir/rejected" source binary input wrong "$scanner_sha" "$validator_sha" "$dispatcher_sha"; then
  echo "dispatcher must reject a mismatched sealed runner before output creation" >&2
  exit 1
fi
test ! -e "$temp_dir/rejected"
if DISPATCH_MARKER="$temp_dir/labels" VALIDATOR_MARKER="$temp_dir/validator-marker" SLICE80_AC072_RUNNER="$temp_dir/runner" \
  SLICE80_AC072_VALIDATOR="$temp_dir/validator" "$campaign" "$root" /bin/true \
  "$temp_dir/campaign" source binary input "$runner_sha" "$scanner_sha" "$validator_sha" "$dispatcher_sha"; then
  echo "dispatcher must fail after an invalid first cell" >&2
  exit 1
fi
test "$(cat "$temp_dir/labels")" = R1.log
test -f "$temp_dir/campaign/R1.log"
test ! -e "$temp_dir/campaign/R2.log"
test "$(cat "$temp_dir/validator-marker")" = invoked

missing_verdict="$temp_dir/missing.json"
if PYTHONPATH="$root" python3 "$root/dev/tools/slice80_ac072_acceptance.py" validate-cell --log "$temp_dir/missing.log" \
  --label R1 --purpose acceptance >"$missing_verdict"; then
  echo "missing raw log must fail closed" >&2
  exit 1
fi
rg -F '"status": "INCOMPLETE"' "$missing_verdict" >/dev/null

echo "ok test_slice80_ac072_campaign_stop"
