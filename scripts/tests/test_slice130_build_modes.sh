#!/usr/bin/env bash
# Native-free environment oracle: cold shipping may fetch its locked graph;
# explicit qualification counterparts must use the selected offline cache.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cat > "$work/probe.sh" <<'PROBE'
trap 'printf "selected-offline=%s\n" "${CARGO_NET_OFFLINE-unset}"' EXIT
PROBE
for wrapper in build-python-cuda-tegra.sh build-napi-cuda-tegra.sh; do
  for mode in shipping qualification; do
    args=(--assert-only)
    expected="unset"
    if [ "$mode" = qualification ]; then
      args+=(--qualification-observer)
      expected=true
    fi
    set +e
    output="$(env -u CARGO_NET_OFFLINE BASH_ENV="$work/probe.sh" bash "$root/scripts/release/$wrapper" "${args[@]}" 2>&1)"
    status=$?
    set -e
    if [ "$status" -gt 1 ] || ! grep -Fxq "selected-offline=$expected" <<<"$output"; then
      printf 'FAIL %s %s forced the wrong cache mode (exit=%s):\n%s\n' "$wrapper" "$mode" "$status" "$output" >&2
      exit 1
    fi
  done
done
printf 'PASS shipping preserves cache policy; explicit counterparts select offline\n'
