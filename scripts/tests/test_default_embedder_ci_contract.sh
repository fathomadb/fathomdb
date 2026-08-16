#!/usr/bin/env bash
# Regression guard for the real-engine default-embedder SDK evidence job.
set -euo pipefail

ci_file=".github/workflows/ci.yml"
job_body="$(awk '
  /^  default-embedder-tests:$/ { in_job = 1; next }
  in_job && /^  [A-Za-z0-9_-]+:$/ { exit }
  in_job { print }
' "$ci_file")"

if [ -z "$job_body" ]; then
  printf 'FAIL default-embedder-tests CI job is absent\n' >&2
  exit 1
fi

require() {
  local needle="$1" description="$2"
  if [[ "$job_body" == *"$needle"* ]]; then
    printf 'PASS %s\n' "$description"
  else
    printf 'FAIL %s\n' "$description" >&2
    return 1
  fi
}

require 'cargo test -p fathomdb-engine --features default-embedder' \
  'Rust real-engine default-embedder test runs after cache warming'
require 'src/ts && npm run build:native:debug' \
  'TypeScript real-engine native binding is rebuilt from this checkout'
require 'python -m pip install -e' \
  'Python binding is installed from this checkout'
require 'src/python/tests/test_use_default_embedder.py' \
  'Python default-embedder open contract runs against the warmed cache'
require 'src/python/tests/test_embed.py' \
  'Python default-embedder inference contract runs against the warmed cache'
