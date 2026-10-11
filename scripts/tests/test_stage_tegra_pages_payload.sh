#!/usr/bin/env bash
# Only exact distribution inputs may cross from Jetson to hosted Pages jobs.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
evidence="$work/evidence"
mkdir -p "$evidence/wheel" "$evidence/node" "$evidence/node-local-smoke"
wheel='fathomdb-0.8.27+tegra-cp312-cp312-linux_aarch64.whl'
main='fathomdb-0.8.27.tgz'
tegra='fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz'
candidate_sha='0123456789abcdef0123456789abcdef01234567'
printf 'wheel\n' > "$evidence/wheel/$wheel"
printf 'main\n' > "$evidence/node/$main"
printf 'tegra\n' > "$evidence/node/$tegra"
printf 'database bytes stay local\n' > "$evidence/cuda.sqlite"
printf 'build log stays local\n' > "$evidence/build-wheel.log"
printf 'native binary stays local\n' > "$evidence/node/native.node"
(cd "$evidence/node" && sha256sum "$main" "$tegra" native.node > SHA256SUMS)
sha256sum "$evidence/wheel/$wheel" > "$evidence/wheel.sha256"
tegra_digest="$(sha256sum "$evidence/node/$tegra" | awk '{print $1}')"
printf '{"outcome":"PASS"}\n' > "$evidence/node-local-smoke/summary.json"
printf '{"source":"local","candidate_sha":"%s","version":"0.8.27","tegra_tarball_sha256":"%s"}\n' \
  "$candidate_sha" "$tegra_digest" > "$evidence/node-local-smoke/context.json"

bash "$root/scripts/release/stage-tegra-pages-payload.sh" \
  --evidence-dir "$evidence" --version 0.8.27 --candidate-sha "$candidate_sha" --out "$work/payload"
find "$work/payload" -type f -printf '%P\n' > "$work/actual-files"
sort -o "$work/actual-files" "$work/actual-files"
mapfile -t actual < "$work/actual-files"
expected=(
  "node/$main"
  "node/$tegra"
  'node/SHA256SUMS'
  "wheel/$wheel"
  'wheel.sha256'
)
printf '%s\n' "${expected[@]}" > "$work/expected-files"
sort -o "$work/expected-files" "$work/expected-files"
mapfile -t expected < "$work/expected-files"
[ "${actual[*]}" = "${expected[*]}" ] || {
  printf 'payload contains unexpected files: %s\n' "${actual[*]}" >&2
  exit 1
}
(cd "$work/payload/node" && sha256sum --check --strict SHA256SUMS)

printf 'tampered\n' > "$evidence/node/$tegra"
if bash "$root/scripts/release/stage-tegra-pages-payload.sh" \
  --evidence-dir "$evidence" --version 0.8.27 --candidate-sha "$candidate_sha" --out "$work/rejected" >/dev/null 2>&1; then
  printf 'stage accepted a tarball that disagrees with the build manifest\n' >&2
  exit 1
fi
printf 'tegra\n' > "$evidence/node/$tegra"
printf '{"outcome":"FAIL"}\n' > "$evidence/node-local-smoke/summary.json"
if bash "$root/scripts/release/stage-tegra-pages-payload.sh" \
  --evidence-dir "$evidence" --version 0.8.27 --candidate-sha "$candidate_sha" --out "$work/rejected" >/dev/null 2>&1; then
  printf 'stage accepted a failed installed-package smoke\n' >&2
  exit 1
fi
printf 'stage-tegra-pages-payload: PASS\n'
