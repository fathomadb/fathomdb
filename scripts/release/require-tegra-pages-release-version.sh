#!/usr/bin/env bash
# Fail closed unless the candidate's project metadata matches the one release
# version that the interim Pages publisher is authorized to serve. A
# `package.json` manifest (the TypeScript package the Tegra Node tarball is
# stamped from) is read as JSON; any other manifest as a pyproject.toml.
set -euo pipefail

manifests=()
expected_version=''

usage() {
  printf 'usage: %s --manifest PYPROJECT|PACKAGE_JSON [--manifest ...] --expected-version X.Y.Z\n' "$0" >&2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest)
      manifests+=("${2:-}")
      shift 2
      ;;
    --expected-version)
      expected_version="${2:-}"
      shift 2
      ;;
    *)
      usage
      exit 64
      ;;
  esac
done

[ "${#manifests[@]}" -gt 0 ] && [ -n "$expected_version" ] || {
  usage
  exit 64
}

read_version() {
  python3 - "$1" <<'PY'
import json
from pathlib import Path
import sys

path = Path(sys.argv[1])
if path.name == 'package.json':
    package = json.loads(path.read_text(encoding='utf-8'))
    if not isinstance(package, dict) or not isinstance(package.get('version'), str):
        raise SystemExit('package manifest lacks a string version')
    print(package['version'])
    raise SystemExit(0)

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib

with path.open('rb') as stream:
    project = tomllib.load(stream).get('project')
if not isinstance(project, dict) or not isinstance(project.get('version'), str):
    raise SystemExit('project manifest lacks a string project.version')
print(project['version'])
PY
}

for manifest in "${manifests[@]}"; do
  [ -f "$manifest" ] || { printf 'project manifest is absent: %s\n' "$manifest" >&2; exit 1; }
  actual_version="$(read_version "$manifest")"
  if [ "$actual_version" != "$expected_version" ]; then
    printf 'Tegra Pages publication requires %s version: expected %s, got %s\n' \
      "$manifest" "$expected_version" "$actual_version" >&2
    exit 1
  fi
done
