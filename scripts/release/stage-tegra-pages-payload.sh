#!/usr/bin/env bash
# Copy only the package bytes needed by the hosted Tegra Pages and driverless jobs.
set -euo pipefail

usage() {
  printf 'usage: %s --evidence-dir DIR --version X.Y.Z --candidate-sha 40-hex --out NEW_DIR\n' "$0" >&2
}

evidence='' version='' candidate_sha='' out=''
while [ "$#" -gt 0 ]; do
  [ "$#" -ge 2 ] || { usage; exit 2; }
  case "$1" in
    --evidence-dir) evidence="$2" ;;
    --version) version="$2" ;;
    --candidate-sha) candidate_sha="$2" ;;
    --out) out="$2" ;;
    *) usage; exit 2 ;;
  esac
  shift 2
done
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] \
  && [[ "$candidate_sha" =~ ^[0-9a-f]{40}$ ]] && [ -d "$evidence" ] && [ -n "$out" ] || {
  usage
  exit 2
}
[ ! -e "$out" ] || { printf 'payload destination already exists: %s\n' "$out" >&2; exit 1; }

shopt -s nullglob
wheel_prefix="$evidence/wheel/fathomdb-$version+tegra-"
wheels=("$wheel_prefix"*-linux_aarch64.whl)
[ "${#wheels[@]}" -eq 1 ] || {
  printf 'expected exactly one %s+tegra wheel, found %s\n' "$version" "${#wheels[@]}" >&2
  exit 1
}
wheel="${wheels[0]}"
main="fathomdb-$version.tgz"
tegra="fathomdb-linux-arm64-gnu-tegra-$version.tgz"
for file in "$wheel" "$evidence/node/$main" "$evidence/node/$tegra" \
  "$evidence/wheel.sha256" "$evidence/node/SHA256SUMS" \
  "$evidence/node-local-smoke/summary.json" "$evidence/node-local-smoke/context.json"; do
  [ -f "$file" ] && [ ! -L "$file" ] || { printf 'required regular file is missing: %s\n' "$file" >&2; exit 1; }
done
source_bytes=$(( $(stat -c %s -- "$wheel") + $(stat -c %s -- "$evidence/node/$main") \
  + $(stat -c %s -- "$evidence/node/$tegra") ))
[ "$source_bytes" -le 1073741824 ] || {
  printf 'Tegra package bytes exceed the 1 GiB transfer limit: %s\n' "$source_bytes" >&2
  exit 1
}

wheel_expected="$(awk 'NR == 1 { print $1 }' "$evidence/wheel.sha256")"
wheel_actual="$(sha256sum "$wheel" | awk '{print $1}')"
[ "$wheel_expected" = "$wheel_actual" ] || { printf 'Tegra wheel differs from build digest\n' >&2; exit 1; }
for name in "$main" "$tegra"; do
  expected="$(awk -v name="$name" '$2 == name { print $1 }' "$evidence/node/SHA256SUMS")"
  actual="$(sha256sum "$evidence/node/$name" | awk '{print $1}')"
  [ "$expected" = "$actual" ] && [[ "$expected" =~ ^[0-9a-f]{64}$ ]] || {
    printf 'Node tarball differs from build digest: %s\n' "$name" >&2
    exit 1
  }
done

python3 - "$evidence/node-local-smoke/summary.json" "$evidence/node-local-smoke/context.json" \
  "$actual" "$candidate_sha" "$version" <<'PY'
import json
from pathlib import Path
import sys

summary = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
context = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
if (summary.get("outcome") != "PASS"
        or context.get("source") != "local"
        or context.get("candidate_sha") != sys.argv[4]
        or context.get("version") != sys.argv[5]
        or context.get("tegra_tarball_sha256") != sys.argv[3]):
    raise SystemExit("installed Tegra Node smoke did not pass on this tarball")
PY

mkdir -p "$out/wheel" "$out/node"
cp -- "$wheel" "$out/wheel/"
cp -- "$evidence/node/$main" "$evidence/node/$tegra" "$out/node/"
(cd "$out/wheel" && sha256sum "${wheel##*/}") > "$out/wheel.sha256"
(cd "$out/node" && sha256sum "$main" "$tegra") > "$out/node/SHA256SUMS"
bytes="$(find "$out" -type f -printf '%s\n' | awk '{total += $1} END {print total+0}')"
count="$(find "$out" -type f | wc -l)"
[ "$count" -eq 5 ] && [ "$bytes" -le 1073741824 ] || {
  printf 'distribution payload must contain five files and at most 1 GiB; found %s files and %s bytes\n' "$count" "$bytes" >&2
  exit 1
}
printf 'Tegra distribution payload: %s files, %s bytes\n' "$count" "$bytes"
find "$out" -type f -printf '%P %s bytes\n' | sort
