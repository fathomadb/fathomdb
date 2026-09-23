#!/usr/bin/env bash
set -euo pipefail

cargo fmt --check
cargo check --workspace
# `AGENT_LONG=1` exercises the spec-conforming long-running variants
# (e.g. AC-021's 60 s schema-error window) as part of the broad CI gate.
AGENT_LONG=1 bash scripts/test-rust-workspace.sh --serial
# The feature-complete gate (every test target the workspace run cannot
# reach, with CUDA and model weights) is opt-in here for runtime reasons.
if [ "${FATHOMDB_FEATURE_COMPLETE:-0}" = "1" ]; then
  bash scripts/test-feature-complete.sh
fi
python3 -m compileall src/python/fathomdb src/python/tests

if [ -x src/ts/node_modules/.bin/tsc ]; then
  (cd src/ts && npm run typecheck)
else
  echo "skipping TypeScript typecheck (run 'cd src/ts && npm install' to enable)"
fi

if command -v mkdocs >/dev/null 2>&1; then
  mkdocs build --strict
else
  echo "skipping MkDocs build (mkdocs not installed)"
fi
