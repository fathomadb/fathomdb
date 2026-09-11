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
#!/usr/bin/env bash
exit 1
EOF
chmod +x "$temp_dir/runner" "$temp_dir/validator"
if DISPATCH_MARKER="$temp_dir/labels" SLICE80_AC072_RUNNER="$temp_dir/runner" \
  SLICE80_AC072_VALIDATOR="$temp_dir/validator" "$campaign" "$root" /bin/true \
  "$temp_dir/campaign" source binary input; then
  echo "dispatcher must fail after an invalid first cell" >&2
  exit 1
fi
test "$(cat "$temp_dir/labels")" = R1.log
test -f "$temp_dir/campaign/R1.log"
test ! -e "$temp_dir/campaign/R2.log"

echo "ok test_slice80_ac072_campaign_stop"
