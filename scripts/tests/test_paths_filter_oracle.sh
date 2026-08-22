#!/usr/bin/env bash
set -euo pipefail

# CI-CP0 exact matcher oracle
# Upstream: https://github.com/dorny/paths-filter
# Commit: fbd0ab8f3e69293af611ebaee6363fc25e6d187d (v4.0.1)
# Adapter: scripts/tests/lib/run-paths-filter-action.sh
# Invocation: the adapter executes the vendored action's declared
#             dist/index.js entry point with a synthetic push event and a real
#             two-commit Git repository. It does not call a local glob proxy.

repo_root="$(git rev-parse --show-toplevel)"
fixture_dir="$repo_root/scripts/tests/fixtures/dorny-paths-filter-v4"
runner="$repo_root/scripts/tests/lib/run-paths-filter-action.sh"

fail() {
  printf 'FAIL test-paths-filter-oracle: %s\n' "$1" >&2
  exit 1
}

read_output() {
  local key="$1"
  local output_file="$2"
  awk -v prefix="$key<<" 'index($0, prefix) == 1 { getline; print; exit }' "$output_file"
}

printf '%s  %s\n' \
  '2a8458c162f898df02eaa4ddad64501a5be7b7d1a27dc788069bcd2219b9a798' \
  "$fixture_dir/dist/index.js" | sha256sum --check --status \
  || fail 'vendored action entry point does not match the pinned checksum'
printf '%s  %s\n' \
  '5730dccb89882cb567a44c9035df1668df1b03c23e4d1a71c59924eeba8e405a' \
  "$fixture_dir/LICENSE" | sha256sum --check --status \
  || fail 'vendored action license does not match the pinned checksum'

metadata_commit="$(node -p "require('$fixture_dir/SOURCE.json').commit")"
[ "$metadata_commit" = 'fbd0ab8f3e69293af611ebaee6363fc25e6d187d' ] \
  || fail 'source metadata does not name the workflow pin'

work_dir="$(mktemp -d "${TMPDIR:-/tmp}/fathomdb-paths-filter-oracle.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

filters="$work_dir/filters.yml"
cat >"$filters" <<'YAML'
python:
  - 'src/python/**'
  - '!src/python/tests/test_slice65_wal_attribution_installed.py'
YAML

windows_output="$work_dir/windows-output"
"$runner" every "$filters" "$windows_output" \
  src/python/tests/test_slice65_wal_attribution_installed.py
windows_python="$(read_output python "$windows_output")"
[ "$windows_python" = false ] \
  || fail 'the exact every matcher included the negated Windows path'

mixed_output="$work_dir/mixed-output"
"$runner" every "$filters" "$mixed_output" \
  src/python/tests/test_slice65_wal_attribution_installed.py \
  src/python/fathomdb/database.py
mixed_python="$(read_output python "$mixed_output")"
[ "$mixed_python" = true ] \
  || fail 'the exact every matcher missed the ordinary Python file in a mixed diff'

printf 'PASS test-paths-filter-oracle\n'
