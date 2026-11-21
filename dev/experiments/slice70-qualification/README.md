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

After the matched diagnosis, `historical-direct` uses the exact repaired EU7
provider from the shared test-support module. It keeps the original fixture's
batching/truncation defaults and validates only the affected stress protocol on
the preserved snapshot. No corpus seeding, recall or passing battery is repeated.

For the selected1000-row stress recovery, run the exact ignored selector
`eu7_repaired_provider_small_stress_only` in `eu7_real_corpus_ac`, with
`AGENT_LONG=1`, the warm original corpus/model, explicit CPU and
`default-embedder,default-reranker,operator,test-hooks`. It reuses canonical
seeding/drain and stress helpers and skips ground truth/recall. Always select
by exact name; the original whole EU7 result remains separately recorded.

## Final platform and archive consumers (Slice75)

The existing exact-candidate native CI matrix runs target-native SDK shared
behavior, CLI diagnostics, installed wheel and release-shaped npm main/platform
consumers (including separate write/reopen processes and strict TypeScript).
It retains packages and CLI separately from JSON native receipts. Linux x64
hostCPU CI remains narrower than the final CUDA-capable shipping pair.

`bash scripts/release/smoke/smoke-source-archive.sh SOURCE.tar FRESH_WORK_DIR`
builds SDK/CLI against the archive's producer lock and tracked source mirrors,
links the maintained SDK consumer from a clean external directory and executes
its write/reopen phases plus `candidate-cli.py` diagnostics. Source file-set,
lock and configuration must remain byte-identical. Registry resolution is a
separate delivery claim.

The manual `CI diagnostic cancellation` workflow emits a complete redacted
250-line failure and its exact101 outcome before a bounded later wait. Cancel
that separate hosted run only after `CANCELLATION_PROBE_READY` appears, then
verify the actual cancelled conclusion and retained completed/started/unrun
scope. The existing offline oversized-line truncation control is separate.
