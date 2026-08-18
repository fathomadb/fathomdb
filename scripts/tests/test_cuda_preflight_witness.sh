#!/usr/bin/env bash
# Behavioral contract for the versioned, SHA-bound CUDA preflight witness.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
exec python3 "$REPO_ROOT/scripts/tests/test_cuda_preflight_witness_v2.py"
