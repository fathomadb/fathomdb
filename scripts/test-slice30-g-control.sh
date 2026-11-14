#!/usr/bin/env bash
# Explicit owned Linux process/cgroup controls; no model, selected store or GPU work.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel)"
cd "$REPO_ROOT"
: "${S30_TASK:?live G controls require an owned task root}"
if [ "${AGENT_SLICE30_LIVE_CONTROLS:-0}" != "1" ]; then
  echo 'select live G controls explicitly with AGENT_SLICE30_LIVE_CONTROLS=1' >&2
  exit 1
fi
export PYTHONPATH="$REPO_ROOT/dev/experiments/0.8.29-slice30-measurements${PYTHONPATH:+:$PYTHONPATH}"
.venv/bin/python - <<'CONTROL'
import os
from pathlib import Path
from g_resource import resource_sample
root, task = Path.cwd(), Path(os.environ["S30_TASK"]).resolve(strict=True)
value = resource_sample(root, task)
GiB = 1024**3
if (value["root_free"] < 32*GiB or value["task_free"] < 250*GiB
        or value["memory_available"] < 16*GiB or value["owned"] >= 200*GiB):
    raise SystemExit("live G control resource start floor fails: " + str(value))
CONTROL
mkdir -p "$S30_TASK/tmp/g-live-controls"
export TMPDIR="$S30_TASK/tmp/g-live-controls"
exec .venv/bin/python -m unittest -v \
  g_phase_tests g_boundary_tests g_boundary_entry_tests g_boundary_dispatch_tests \
  g_guarded_entry_tests g_phase_error_tests g_closed_guard_tests \
  g_closed_uncommitted_tests g_closed_cell_cli_tests
