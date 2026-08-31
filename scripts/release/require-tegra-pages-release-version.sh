#!/usr/bin/env bash
# Fail closed unless the candidate's Python project metadata matches the one
# release version that the interim Pages publisher is authorized to serve.
set -euo pipefail

manifest=''
expected_version=''

usage() {
  printf 'usage: %s --manifest PYPROJECT --expected-version X.Y.Z\n' "$0" >&2
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest)
      manifest="${2:-}"
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

[ -n "$manifest" ] && [ -n "$expected_version" ] || {
  usage
  exit 64
}
[ -f "$manifest" ] || { printf 'project manifest is absent: %s\n' "$manifest" >&2; exit 1; }

actual_version="$(python3 - "$manifest" <<'PY'
from pathlib import Path
import sys

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib

with Path(sys.argv[1]).open('rb') as stream:
    project = tomllib.load(stream).get('project')
if not isinstance(project, dict) or not isinstance(project.get('version'), str):
    raise SystemExit('project manifest lacks a string project.version')
print(project['version'])
PY
)"

if [ "$actual_version" != "$expected_version" ]; then
  printf 'Tegra Pages publication requires project version: expected %s, got %s\n' \
    "$expected_version" "$actual_version" >&2
  exit 1
fi
