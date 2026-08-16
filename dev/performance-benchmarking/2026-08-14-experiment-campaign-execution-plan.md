# Out-of-band experiment campaign — execution plan

> **Program relationship.** This runbook executes C0, L0/L1, T0, and F0 from
> the [overall performance benchmarking and experiments program](PROGRAM.md).
> Use that program for cross-track priority and blockers; use this document for
> the concrete campaign sequence.

## Boundary

This is the execution runbook for the experiments campaign. Every item is
out-of-band: a result may inform a later decision, but it does not complete,
block, or become a gate for a numbered release without a separate HITL ruling.
Raw artifacts remain in the external experiment store; Git holds safe receipts,
summaries, and the generated experiment index/scoreboard only.

## Execution sequence

1. **Campaign controls — complete.** Use the persistent external artifact
   root, safe receipt projection, append-only experiment index, and scoreboard.
   Preserve raw corpus, predictions, database files, and logs outside Git.

2. **LOCOMO canonical A0 — complete.** Retain the full historical-parity,
   turn-level FTS top-10 run as A0. Its fixed-seed bootstrap-derived M1 margin
   is the screening baseline; it is not a winner claim.

3. **LOCOMO Phase-B grid.** Run turn and session ingest units through FTS,
   hybrid dense+FTS, the three pre-registered CE settings, and bounded
   source-aware neighbor expansion. Record M1/M2, temporal proxy, ingest and
   query timing, provenance, and safe receipts. Prune only by the
   pre-registered M1 paired-margin and Fast-profile latency rules.

4. **GPU enablement for GPU arms.** Before any dense/CE GPU configuration,
   build a dedicated CUDA-feature Python extension from canonical main and
   prove the extension actually imported plus initialized CUDA. CPU/FTS arms
   may proceed independently. GPU result receipts identify the device and
   loaded extension; a CPU fallback is not labelled a GPU result.

5. **LOCOMO shortlist.** Rank Phase-B survivors by M1 then M2 and select at
   most A0, the best Fast profile, and the best Quality-GPU profile. Prove
   façade retrieval-fingerprint parity before deciding whether A0's historical
   judge score may be reused.

6. **LOCOMO M3/M4.** Use the already-cleared Airlock loopback route only for
   the shortlist, with one worker, checkpoint/resume, a declared spend cap,
   and safe receipts. This supplies actual answer correctness and temporal
   correctness; the Phase-B temporal result remains a retrieval proxy.

7. **LOCOMO comparator work.** Run Mem0 only against selected FathomDB
   winner(s), using the pre-registered paired confidence rule. Any product or
   escalation work is conditional on this result; it is not part of the free
   grid.

8. **TC-5 eu7 grown-corpus characterization.** First implement the
   manifest-backed, no-synthetic-padding characterization path described in
   `2026-08-14-eu7-tc5-scale-envelope-rebaseline-plan.md`. Then measure the
   CPU same-backend 7,667-document bridge and the all-real 18,472-document
   primary arm. It produces an advisory fidelity envelope and a HITL choice,
   never a release verdict.

9. **F-17 soft scale-bound characterization.** Run a separate, out-of-band
   experiment over the complete current exact-search local-first surface. The
   purpose is to characterize and document a *soft* supported-scale envelope,
   not to promise capacity or justify ANN work. It consumes TC-5's frozen eu7
   fidelity result and existing latency/stress measures as inputs.

   - **Pre-register the workload matrix.** Enumerate each operation/profile
     claimed in the envelope—at minimum open, ingestion/drain, FTS retrieval,
     pre-fusion vector retrieval, hybrid retrieval, and any enabled reranker.
     A surface without a measured workload is excluded from the envelope.
   - **Freeze the scale ladder before measuring.** Use the existing 10k point,
     the real 18,472-document TC-5 point where applicable, and intermediate
     points through the observed exact-search crossover (25k, 40k, and 50k
     rows). Synthetic/vector fixtures may characterize latency above the real
     corpus size, but are never represented as eu7 fidelity evidence.
   - **Separate measurement axes.** Fidelity is TC-5's CPU same-backend
     all-real eu7 result. Latency and mixed-load behavior use the existing
     AC-013/AC-019-style measures with recorded hardware and configuration.
     CPU, CUDA, and reranker-enabled profiles are distinct rows; do not merge
     or extrapolate across them.
   - **Make the boundary observed, not assumed.** For every matrix cell record
     p50/p95/p99, throughput, seed/open cost, stress behavior, errors, corpus
     or fixture provenance, and result completeness. Report the largest
     measured configuration that satisfies its explicitly named advisory
     criteria; do not interpolate beyond 50k or call an unmeasured operation
     supported.
   - **Publish an advisory only.** The F-17 receipt and report state the
     exact workloads, corpus/fixture hashes, host/device, engine/model
     identities, measured range, and exclusions. They explicitly say that the
     O(N) exact-search path's roughly-50k crossover is characterization
     context, not a hard capacity guarantee. A later HITL decision is required
     for any stated or hard support bound.

## Completion order

Steps 3 and 8 may proceed independently once their respective test/manifest
preconditions are met. Step 4 gates only GPU LOCOMO cells. Steps 5–7 wait for
the Phase-B grid. Step 9 waits for TC-5's primary fidelity receipt and for its
own pre-registered workload matrix; it does not wait for paid M3/M4 scoring.
