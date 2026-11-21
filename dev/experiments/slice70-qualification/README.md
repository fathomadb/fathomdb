---
title: Slice 70 source qualification commands
status: ACTIVE
---

# Slice 70 qualification

The [plan](../../plans/0.8.30/features/slice-70/plan.md) and
[design](../../plans/0.8.30/features/slice-70/design.md) preserve exact existing
canonical selectors, thresholds and evidence reuse. Raw execution logs remain
outside the checkout under `/tmp/fathomdb-0830-slice70-evidence`.

`run_orin_tc5.py CONFIG --output /tmp/fathomdb-0830-slice70/OUTPUT` selects only
the frozen real TC5 bridge through the maintained GPU runner. Use the configured
owned interpreter and wheel; it reuses the existing Orin exclusive lock and
observed-peer/resource helpers. Sampling at one-second cadence is a resource
check, not a100ms fault-protocol or latency claim. No reset is performed.

For an observation-only failure after a complete ingest, `--resume-from PRIOR`
reuses only its byte-pinned complete database with no uncheckpointed WAL.
The maintained runner validates the full first-query input/model/binary binding
and corpus counts, then copies to a new root and runs all100 queries afresh.
Record this as assembled proof and retain the original failed cell; no ingest
or already passing native qualification is repeated.
The original ingest's wheel and candidate-configuration identity must be retained
independently. The first-query manifest pins model/index/executor inputs; it is
not by itself an attestation of which Python wheel performed the ingest.

If a narrow executor repair changes its digest, `--prior-executor OLD_BINARY`
binds the original first-query manifest to that preserved binary, while the
current candidate configuration independently pins the new query executor.
The option requires `--resume-from`. Each query opens its own hash-checked copy
of the unchanged ingest because Engine open may update operational metadata.
A typed nonmeasurement stops immediately. Failed cells remain failed.

`slice70_stress_compare` is a private engine example, compiled with
`default-embedder,test-hooks` in release mode. Invoke `MODE SNAPSHOT SHA256
FRESH_DIRECTORY`, with mode `serialized` or `direct`, explicit CPU and an empty
`CUDA_VISIBLE_DEVICES`. Use a separate process/directory for each mode and the
same drained7667-record snapshot/model/corpus. It never seeds. It preserves the
canonical100 texts,15-candidate override/default10 results,1000 warmups,
1000 timed baseline and8x250 timed stress. Both modes delegate identity,
truncation and batching identically; only the full-forward mutex differs.
Every search validates unchanged ordered hits and no fallback/degradation after
capturing elapsed time. Save and compare `witnesses.json` across arms, including
pre-fusion vector participation, before interpreting the printed JSON timing.
No result waives or replaces the canonical failure; unfair-lock attribution
would require separate lock-wait evidence.

`complete_tc5_bridge.py` is the fixed recovery for the77 measured queries and
23 missing ordinals retained by this slice. It reconstructs every frozen query
manifest/spec and checks each result's settings, query, selection, fixture,
index, model, runtime and executor identity before the existing bootstrap.
It executes only078–100, each as the guard's native leader with an unreaped PID,
120seconds per cell and one shared30-minute deadline. TERM/INT forwards to the
active guard and awaits cleanup. Unknown peers still abort. Original wrapper
failures remain; any completed result is assembled fidelity proof only.
