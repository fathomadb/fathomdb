---
title: Slice 135 current installed TypeScript S02 functional refresh
status: FUNCTIONAL_REFRESH_NO_LATENCY_VERDICT
target_release: 0.8.27
---

# Current installed TypeScript S02 functional refresh — 2026-10-07

The [S02 sequence](../../../../../../../scripts/slice135_ts_s02.mjs) passed
through the installed TypeScript package and Linux native addon built from
product source `ddd05221a5dd38bd076bfad48e7f57717d8af791`. The current
Slice 135 branch has no later changes in the TypeScript-facing engine,
query, schema, N-API or TypeScript source trees. The source, package archive,
installed module and native bytes are bound in [audit.json](audit.json) and
the [capability receipt](../2026-10-07-ts-capability-exercise/README.md).
The package version string remains 0.8.26.

One fresh real SQLite database passed open, governed write, ready vector
projection, text/vector/hybrid retrieval, graph/evidence resolution, source
erasure and retry, close/reopen, and post-reopen state checks. The
[raw result](raw.json) retains 18 stage durations and materialized
observations. The independent [S02 observation checker](../2026-10-07-ts-s02-feasibility/audit.py)
accepted anchor, evidence, erasure and canonical counts, verified the
installed package files against both retained npm archives, and rejected a
negative control that changed the reopened graph-edge count from zero to one.

The successful command from the Slice 135 checkout was:

```sh
env -u NODE_PATH /home/coreyt/.nvm/versions/node/v25.9.0/bin/node \
  scripts/slice135_ts_s02.mjs \
  --install-root /tmp/slice135-ts-capability-sealed/consumer \
  --source-sha ddd05221a5dd38bd076bfad48e7f57717d8af791 \
  --expected-native-sha256 2d26e58cda598d702c3dc333beaeec355e268c093c342fbf1efd26e4ccd5186d \
  --output /tmp/slice135-ts-s02-current-functional.json
```

The raw output was copied byte-for-byte to this directory. To independently
repeat the retained artifact and state checks from the checkout root:

```sh
python3 - <<'PY'
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import runpy
import tarfile

root = Path('dev/plans/0.8.27/features/slice-135/results')
receipt = root / '2026-10-07-ts-s02-current-refresh'
packages = root / '2026-10-07-ts-capability-exercise'
raw = json.loads((receipt / 'raw.json').read_text())
expected = json.loads((receipt / 'audit.json').read_text())
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
assert sha(receipt / 'raw.json') == expected['raw_sha256']
assert raw['source_sha'] == expected['source_sha']
assert sha(Path('scripts/slice135_ts_s02.mjs')) == raw['runner_sha256']
assert sha(Path('scripts/slice135_ts_s01.mjs')) == raw['s01_helper_sha256']
main = packages / 'fathomdb-0.8.26.tgz'
native = packages / 'fathomdb-linux-x64-gnu-0.8.26.tgz'
assert sha(main) == expected['main_archive_sha256']
assert sha(native) == expected['native_archive_sha256']
for archive, member, key in (
    (main, 'package/dist/index.js', 'module_sha256'),
    (main, 'package/package.json', 'package_sha256'),
    (native, 'package/fathomdb.linux-x64-gnu.node', 'native_sha256'),
):
    with tarfile.open(archive, 'r:gz') as package:
        stream = package.extractfile(member)
        assert stream is not None
        digest = hashlib.sha256(stream.read()).hexdigest()
    assert digest == raw['artifact'][key]
check = runpy.run_path(str(root / '2026-10-07-ts-s02-feasibility/audit.py'))['check_observed']
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
print('current TypeScript S02 receipt valid')
PY
```

The one whole duration is **5,239.757 ms including validation reads**. It is
functional evidence only. No baseline-only S02 noise pilot, qualified whole
timer, alternating comparison blocks or contention condition ran. It cannot
establish a latency change or complete the Phase 1 checkpoint.
