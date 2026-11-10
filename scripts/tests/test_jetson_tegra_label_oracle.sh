#!/usr/bin/env bash
# Check the evidence validator against the accepted registered-runner inventory.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
validator="$root/scripts/tests/test_jetson_tegra_cuda_evidence_ci_job.sh"
config="$root/.github/actionlint.yaml"
failed=0
if ! ACTIONLINT_CONFIG="$config" bash "$validator" >"$work/positive.log" 2>&1; then
  cat "$work/positive.log"
  echo 'FAIL label oracle rejected the accepted five-label configuration' >&2
  failed=1
fi
for label in gpu cuda-12 jetson aarch64 windchill3-windows-11; do
  python3 - "$config" "$work/missing.yaml" "$label" <<'PY'
from pathlib import Path
import sys
text=Path(sys.argv[1]).read_text()
label=sys.argv[3]
needle=label+', ' if label!='windchill3-windows-11' else ', '+label
assert text.count(needle)==1
Path(sys.argv[2]).write_text(text.replace(needle,''))
PY
  if ACTIONLINT_CONFIG="$work/missing.yaml" bash "$validator" >"$work/negative.log" 2>&1; then
    echo "FAIL label oracle accepted missing registered label: $label" >&2
    failed=1
    continue
  fi
  grep -Fq 'FAIL  actionlint knows the registered custom Jetson labels' "$work/negative.log" || {
    cat "$work/negative.log"
    echo "FAIL missing-label control failed for an unrelated reason: $label" >&2
    failed=1
  }
done
[ "$failed" -eq 0 ]
echo 'PASS accepted five-label inventory and every missing-label control'
