# SCALE-02 — Local-first scale envelope

**Status:** diagnostic continuation complete; the original fixed-policy A0
envelope remains 17,272, configured-as-is passes the scale-adjusted policy at
25k, and the mmap hypothesis passes it at 40k but not 50k.

## Decision

What measured local-first operating envelope is supportable for the selected
retrieval profile without implying an unmeasured capacity guarantee?

## Inputs and scope

- The selected profile is A0: turn-level FTS, `top_k=10`, with no embedder or
  reranker. The [ANSWER-01 result](answer-01-shortlist-scoring.md) retains A0.
- The
  [SCALE-01 primary](../../../experiments/runs/tc5-gpu-primary-20260822T1605Z-2d574205/record.json)
  qualifies pre-fusion vector fidelity at 17,272 documents only. It does not
  select vector, hybrid, or reranking for A0 or establish their efficiency.
- SCALE-02 measures FathomDB 0.8.23 on one declared host. Its result is an
  advisory operating envelope for the registered workload, not a product
  capacity limit.
- The executable
  [configuration](../../../experiments/configs/scale-02/a0-envelope.v2.json)
  binds the selected production path, workload, runtime, and advisory policy
  approved by the HITL on 2026-08-22.
- The HITL subsequently authorized `seq-266`: preserve the fixed-policy result,
  collect configured-as-is 40k and 50k baselines, and separately evaluate a
  scale-adjusted p50 budget of `max(20 ms, records / 1000)` (25/40/50 ms at
  25k/40k/50k). The executable continuation contract is
  [`post-boundary-baseline.v1.json`](../../../experiments/configs/scale-02/post-boundary-baseline.v1.json).

## Follow-up inputs

- Preserve the existing [10k baseline receipt](../../../experiments/runs/scale-02-a0-10000-20260822T1715Z-77c37c77/record.json),
  approved v1 configuration, ANSWER-01 A0 receipt, and SCALE-01 receipt by
  path, run ID, and SHA-256.
- Before implementation, copy the qualified TC-5 corpus pack, its 100-query
  set, and the ANSWER-01 LOCOMO corpus and fixed 32-question subset from their
  temporary roots into
  `data/performance-benchmarking/scale-02/inputs/input-pack-v1/`. Preserve
  relative paths and write `input-manifest.v1.json` with every file's size and
  SHA-256. A safe committed input-pack receipt records only aggregate counts,
  digests, and the persistent-root identifier.
- Every executable configuration pins the input-manifest digest, query order,
  seeds, host facts, code SHA, binary and extension digests, SQLite version,
  exact reader settings, and the approved CPU-only A0 route. No `auto`, GPU,
  embedder, reranker, or paid model belongs in these cells.

## Stored evidence contract

- Raw databases, per-query timings, query-plan evidence, doctor output, logs,
  and content-bearing equivalence data live under
  `data/performance-benchmarking/scale-02/runs/<run-id>/`. Each run writes an
  `artifact-manifest.v1.json` containing relative paths, byte sizes, and
  SHA-256 values. Keep these artifacts through program close.
- Safe aggregate evidence is committed through the existing
  `experiments.record.v1` receipt and append-only index. Configurations live in
  `experiments/configs/scale-02/`; receipts live in `experiments/runs/`.
- Planned typed payloads are `scale-02-fts-tuning.v1` for tuning,
  `scale-02-fts-selection.v1` for synthesis, and `scale-02-execution.v2` for
  the formal rerun. Their implementations must reject unknown fields,
  repository artifact roots, missing digests, reused databases, and requested
  versus observed reader-setting drift.

## Follow-up plan

All four steps are complete. The HITL approved the step 3 recommendation, and
step 4 stopped at the first failing point as registered.

1. **Implement and qualify the candidates.** Add the deterministic FTS rank
   fast path: request the existing direct-text candidate window plus one row
   with `ORDER BY rank`, sort the retained candidate window by
   `(score, write_cursor)`, and fall back to the existing full sort when the
   candidate-window boundary is tied. Test byte-identical ordered hits and
   scores for overfetch/body deduplication, ties, supersession, validity, edge
   fusion, and public limit-prefix behavior.
   Implement observed reader-setting evidence; experiment-only environment
   hooks are not an eligible production configuration.
2. **Generate tuning and equivalence evidence.** First compare the current and
   rank-fast query paths under shipped reader defaults. On the eligible query
   path, compare default readers, 128 MiB mmap, 256 MiB mmap, 64 MiB cache per
   reader, and 256 MiB mmap plus 64 MiB cache. Use five fresh 10k databases per
   performance cell, with the v1 cold/warm/steady workload unchanged. For the
   selected reader candidate, separately measure concurrency 1, 2, and 4;
   concurrency 1 remains the formal latency treatment. Emit a content-free
   equivalence receipt over the 100 TC-5 queries and ANSWER-01's 32 A0 queries.
   `temp_store=MEMORY`, FTS `optimize`, tokenizer/schema changes, and query-term
   narrowing remain excluded unless the registered cells fail and a new plan
   justifies them.
3. **Analyze and propose the formal configuration.** Write one
   `scale-02-fts-selection.v1` receipt that consumes the baseline, tuning,
   concurrency, equivalence, ANSWER-01 quality, and SCALE-01 fidelity receipts
   by run ID and digest. A candidate is eligible only with identical ordered
   top-10 retrieval evidence, zero errors/timeouts, complete repetitions,
   unchanged quality applicability, and 95% upper bounds within the original
   20 ms p50, 150 ms p99, and 80% RAM policy. Recommend the lowest-footprint
   eligible production setting, state every rejection, and produce an exact
   `a0-envelope.v2` proposal. The selection receipt remains
   `recommendation_pending_hitl`; it is not permission to rerun.
4. **Rerun formal SCALE-02 at 10k only after approval.** After the HITL approves
   the selection and v2 configuration together, land the selected behavior as
   the measured production default, pin its build digests, dry-run the frozen
   inputs, and execute the original five-repetition 10k contract in fresh
   databases. Resume 17,272-to-50k only if the formal 10k receipt passes.

## Decision rule

Name the largest directly measured point at which every required A0 cell meets
the pre-registered advisory criteria. Report optional component envelopes
separately: their success does not enlarge A0, and their failure does not
shrink A0 unless it affects a required A0 cell. Keep cold and steady-state,
CPU and GPU, and canonical and derived-row results distinct. Link SCALE-01 as
fidelity evidence without pooling it into SCALE-02 performance results.

## Result

The [10k receipt](../../../experiments/runs/scale-02-a0-10000-20260822T1715Z-77c37c77/record.json)
records five complete fresh-database repetitions with zero errors and zero
timeouts. Steady FTS p50 was 29.94 ms against the approved 20 ms limit; p99 was
58.93 ms against 150 ms. The first point therefore failed advisory eligibility,
and the registered stop rule prohibited the 17,272-to-50k points. No passing
SCALE-02 operating point was established under this policy.

Unregistered local diagnostics motivated the follow-up matrix but are not
decision evidence. They must be reproduced by the stored tuning receipts.

The corrected [compact tuning receipt](../../../experiments/runs/scale-02-fts-tuning-20260822T1851Z-51e41245/record.json)
records five fresh repetitions per cell, zero errors/timeouts, exact ordered
top-10 equivalence for all 100 TC-5 and 32 ANSWER-01 queries, and observed
reader settings. `rank_default` is the lowest-footprint eligible cell: its 95%
upper bounds are 9.50 ms p50 and 44.25 ms p99 with shipped reader defaults.
The [selection receipt](../../../experiments/runs/scale-02-fts-selection-20260822T1859Z-946ebdc2/record.json)
recommended `rank_default`. The HITL approved that recommendation and its
production landing as `seq-265` in the program decision ledger. The
[result summary](../2026-08-22-scale-02-fts-followup-result.md) records the
results and trade-offs, and the
[approved v2 configuration](../../../experiments/configs/scale-02/a0-envelope.v2.json)
pins the production runtime.

The formal [10k](../../../experiments/runs/scale-02-a0-10000-20260822T2239Z-e993dd61/record.json)
and [17,272](../../../experiments/runs/scale-02-a0-17272-20260822T2241Z-c543eeb9/record.json)
points passed every registered criterion. The
[25k point](../../../experiments/runs/scale-02-a0-25000-20260822T2245Z-ee93a826/record.json)
failed only steady p50 at 23.90 ms against the 20 ms policy. All attempted
points completed five repetitions with zero errors and zero timeouts. The stop
rule prohibited 40k and 50k, so the measured advisory envelope is 17,272.

## Stop

Stop at the first unsafe point, breached resource limit, persistent error under
the registered retry rule, incomplete repetition, provenance/device mismatch,
or reused database. Do not advance through a failed size, extrapolate between
points or past 50k, substitute synthetic rows for the registered corpus, or
convert the advisory result into a capacity guarantee.

The `seq-266` diagnostic continuation is the sole exception to advancing past
the fixed-policy failure. It changes no runtime, workload, corpus construction,
or original receipt. Its 40k and 50k outputs are post-boundary baselines, not an
expansion of the original envelope. At most two accuracy-preserving tuning
hypotheses may be frozen after those baselines are analyzed and before their
25k/40k/50k cells run.

That continuation is complete. The
[result](../2026-08-22-scale-02-scale-extension-result.md) and
[hypothesis receipt](../../../experiments/runs/scale-02-scale-hypotheses-20260822T2328Z-55ce25d2/record.json)
record the configured-as-is baselines and both frozen hypotheses. No tested
profile passes at 50k. A
[third off-shoot](../2026-08-22-scale-02-rank-boundary-result.md) completed its
two-factor test of streamed BM25 boundary-tie completion under default and
mmap128 readers. Both streamed cells pass through 50k with exact ordered
top-100 and top-10 equivalence. The registered selector recommends shipped
reader defaults. HITL `seq-268` approves `stream_default` as the production FTS
path and retains the no-mmap decision. The production code is landed and
code-verified without a confirming benchmark; see the
[implementation note](../2026-08-23-scale-02-stream-default-implementation.md).
