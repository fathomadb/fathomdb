#!/usr/bin/env bash
# Explicit production qualification; intentionally absent from fast/heavy/all.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
"$REPO_ROOT/scripts/check-module-boundaries.sh"
exec python3 "$REPO_ROOT/scripts/tests/module_boundary_gate_qualification.py" "$REPO_ROOT"
