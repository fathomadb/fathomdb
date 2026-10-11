---
title: Slice 135 current installed Python S02 functional refresh
status: FUNCTIONAL_REFRESH_NO_LATENCY_VERDICT
target_release: 0.8.27
---

# Current installed Python S02 functional refresh — 2026-10-07

The [S02 sequence](../../../../../../../scripts/slice135_python_s02.py)
passed through the rebuilt installed Python wheel after the node/edge search,
graph row-error and Python frozen-error repairs. Its source build commit is
`8587b0571f4e6477045e60ddd341b0cd28f5a3cb`; the same product changes
were integrated into the Slice 135 branch. The [wheel and native binary](../2026-10-07-python-frozen-error-fix/README.md)
are SHA-bound in [audit.json](audit.json). The package still declares version
0.8.26, so the version string alone is not source identity.

One fresh real SQLite database passed open, 36 governed writes, projection
drain, text/vector/hybrid retrieval, graph/evidence resolution, source erasure
and idempotent retry, close/reopen, and post-reopen state checks. The
[raw observation](raw.json) records 18 stage durations and the materialized
results. The independent S02 [observation checker](../2026-10-07-python-s02-feasibility/audit.py)
accepted the exact anchor, evidence, erasure and canonical counts, verified
the wheel's native bytes and runner/helper hashes, and rejected a negative
control that changed the reopened graph-edge count from zero to one.

The run used a new `/tmp` venv with only the retained wheel installed:

```sh
env -u PYTHONPATH -u VIRTUAL_ENV PYTHONDONTWRITEBYTECODE=1 timeout 180s \
  /tmp/slice135-python-s02-current-venv/bin/python \
  scripts/slice135_python_s02.py \
  --wheel dev/plans/0.8.27/features/slice-135/results/2026-10-07-python-frozen-error-fix/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  --wheel-sha256 813255e452a5a001fca5a0943412309a277fff67d6128544cfb8850d87dffe38 \
  --source-sha 8587b0571f4e6477045e60ddd341b0cd28f5a3cb \
  --output /tmp/slice135-python-s02-current-functional.json
```

The exact raw output was copied byte-for-byte to this directory and its
SHA-256 was checked after copying. To repeat the independent semantic and
negative-control check from the Slice 135 worktree root:

```sh
python3 - <<'PY'
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import runpy
import zipfile

root = Path('dev/plans/0.8.27/features/slice-135/results')
receipt = root / '2026-10-07-python-s02-current-refresh'
wheel = root / '2026-10-07-python-frozen-error-fix/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl'
raw = json.loads((receipt / 'raw.json').read_text())
expected = json.loads((receipt / 'audit.json').read_text())
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
assert sha(receipt / 'raw.json') == expected['raw_sha256']
assert sha(wheel) == raw['artifact']['wheel_sha256'] == expected['wheel_sha256']
assert sha(Path('scripts/slice135_python_s02.py')) == raw['runner_sha256']
assert sha(Path('scripts/slice135_python_s01.py')) == raw['s01_helper_sha256']
with zipfile.ZipFile(wheel) as archive:
    assert hashlib.sha256(archive.read('fathomdb/_fathomdb.abi3.so')).hexdigest() == expected['native_sha256']
check = runpy.run_path(str(root / '2026-10-07-python-s02-feasibility/audit.py'))['check_observed']
check(raw['observed'])
bad = deepcopy(raw['observed'])
bad['after_reopen']['canonical_counts']['graph_edges'] = 1
try:
    check(bad)
except ValueError as error:
    assert str(error) == expected['negative_control_rejected']
else:
    raise AssertionError('negative control accepted')
assert raw['whole_sequence_including_checks_ns'] >= sum(raw['stage_ns'].values())
assert len(raw['stage_ns']) == expected['stage_count']
print('current Python S02 receipt valid')
PY
```

The one whole duration is **5,377.039 ms
including validation reads**. It was run while other isolated builds were
active. It is only a functional refresh: no baseline-only S02 noise pilot,
qualified timing boundary, alternating paired blocks, or contention condition
ran. It cannot establish a latency change. The full Phase 1 checkpoint still
requires those cells and a final candidate source check.
