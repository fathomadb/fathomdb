# Tegra private-pool diagnostic

Slice 95 CI22's one preregistered local investigation is retained at source
`395e1a99dc9477e7eb6783d3599ef907b532a016`, including this runner and its
`fathomdb.tegra-gpu-pool-diagnostic/v1` output. Check out that exact source and
use its matching sealed addon to reproduce the historical v1 investigation.
Its original negative v1 result remains a failure; later records do not relabel it.

Current source emits allocation witness v2 and the separate developer diagnostic
`fathomdb.tegra-gpu-pool-diagnostic/v2`. This diagnostic preserves the primary
witness result, including errors. It adds no OpenReport or SDK field and does not
supply an acceptance verdict. Hardware execution requires an approved experiment.

`run.sh` runs the unchanged maintained installed Node row once. Prepare a fresh
consumer containing the exact candidate main/Tegra packages and bind their native,
package and source hashes externally. Then, from the matching repository source:

```sh
bash dev/experiments/tegra-pool-diagnostic/run.sh "$CONSUMER" "$NEW_EVIDENCE_DIR"
```

Use the pinned Node 25.9.0 environment. The runner preserves the child PID, exact
command, row source hash, stdout/stderr, row JSON and original exit status. A
primary witness failure can leave diagnostic observations while still exiting 1.
INT/TERM forwards to and waits for only the owned consumer, retaining partial logs.
Do not retry it to obtain a pass.

The controlled window is the single awaited `Engine.open`. The driver launches no
concurrent SDK/GPU operation or additional worker during that window. Native N-API
and runtime threads still exist; this is not an OS-single-thread guarantee. The
unchanged row can call report/embed/close afterwards, outside the witness window.
No general runtime exclusivity guarantee is claimed.

The private `FATHOMDB_GPU_POOL_DIAGNOSTIC=1` switch is effective only inside an
already requested CUDA allocation witness. Other values leave it off. It emits one
`fathomdb-tegra-pool-diagnostic` stderr line containing the primary outcome and
partial failure observations. Successful forward dimension is retained even when
later primary control or evaluation fails. Opt-out performs no extra probe,
control or diagnostic output.

The session holds the actual witnessed stream, context and private pool. Identity
includes PID, ordinal, UUID, checked context ID as lossless text and raw handles.
Primary global samples remain truthful, including zero/negative deltas. Current
private primary model/control observations are retained by the witness record;
the diagnostic records its additional control separately. Primary reads are
USED then global; extra diagnostic reads are USED, RESERVED then global. All are
sequential, not atomic. Only completed private brackets contribute the retained
model-global pair. Early private failures retain available identity, primary
error and forward dimension if completed; this is not a complete trace of
partial model-bracket counters. Source `395e1a99d` retains the original CI22
experiment's observation behavior. A non-private allocator is unsupported
for this extra diagnostic and receives no diagnostic control.

Only after the original primary observations, checked model load, real forward
and computed witness verdict does the still-live model's pool receive the extra
64 MiB developer control on that same stream. It is touched nonzero, completed and
measured; only that control is dropped, then completion and exact USED-baseline
restoration are checked. Diagnostic errors never replace the primary result.
No trimming, pool-policy change, sleep, row reordering, retry or clamping occurs.

Neither `diagnostic_valid` nor RESERVED/high-water/global values substitute for
raw current-witness acceptance. The current allocation witness contract is the
technical successor `ADR-0.8.28-attributable-cuda-allocation-witness.md`; this tool
remains an optional investigation tool, not a release qualifier.

Focused process checks use a fake Node executable and never import a GPU package:

```sh
python3 dev/experiments/tegra-pool-diagnostic/test_runner.py
```
