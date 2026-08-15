# LOCOMO FathomDB capability campaign — revised

## Phase A gates

1. Checkpoint the recorder code/test diff only; exclude `logs/`, raw artifacts, and unreviewed run directories. Create an isolated campaign worktree from that commit, reconcile it with current `origin/main`, and record checkpoint, main, dependency-lock, harness, and wheel hashes.

2. Locate and preserve historical Fathom/Mem0 receipts and external scored outputs. Identify the ingest unit used by the historical FTS run; pin that exact configuration as canonical **A0** for parity checking and Phase B.4, regardless of whether the alternate FTS ingest unit later ranks better. Label the prior FTS result as historical; never overwrite it.

3. Add façade provenance mapping from normalized payload fingerprint to stable conversation/session/turn IDs. Reject unmapped or ambiguous payloads. Return safe evaluation provenance without exposing raw corpus text.

4. Add M6/M7 instrumentation: façade and engine query p50/p95/p99; ingest acknowledgement and ready-to-search timings; cold-load metrics separate from steady state.

5. Make Airlock a hard preflight:
   - The user-level systemd service is active and its safe loopback liveness endpoint, `127.0.0.1:4000/health/liveliness`, returned HTTP 200.
   - Before any M3 work, verify the configured OpenAI-compatible loopback endpoint, authenticated model-alias probe, and no direct-provider credential route.
   - The scorer uses the verified endpoint only, honors `Retry-After`, checkpoints externally, defaults to one worker, and records modeled cumulative spend.

6. Build/reuse GPU support safely: create the GPU-capable wheel only from the canonical main checkout, install that pinned wheel into a dedicated external campaign environment, and record its hash. Never run `maturin develop` or editable installs from the campaign worktree.

## Measurement contract

- M1 gate is only R@10. Before screening, derive its pre-registered margin as `δ = 2 × SE_bootstrap(A0 R@10)` using 10,000 fixed-seed bootstrap resamples of A0’s per-question results.
- Record `δ`, seed, and bootstrap count, then sanity-check that δ falls within 1–4 percentage points. Outside that range, halt the grid and obtain HITL confirmation rather than applying a degenerate cutoff.
- A candidate passes when the one-sided paired 95% CI lower bound for `candidate − A0` is at least `−δ`. Report R@5/R@20 but do not gate on them.
- M2 is MRR, r@1, and nDCG@10.
- During the free grid, do not claim actual M4. Record `M4_proxy.temporal_evidence_recall` for temporal-class M1-style retrieval only, and mark judge-scored M4 as unmeasured.
- Actual M4 is temporal answer correctness and is computed only with M3 on the paid shortlist.
- Fast-local means M1 pass plus p95 ≤ `1.5 × A0 p95`. The 1.5× upper bound preserves room for bounded neighbor expansion while remaining within HITL’s fast-local range.
- Quality-local-GPU reports latency without treating the 1M-scale 3–8 ms band as a LOCOMO acceptance claim.

## Phase B — self-characterization

Run both ingest units—individual turns and complete sessions—through:

- FTS-only top-10; the historical ingest unit is canonical A0, while the other is an independently ranked candidate.
- Hybrid dense+FTS top-10.
- Hybrid+CE `(α=.3, pool=10, depth=10, candidates=10)`.
- Hybrid+CE `(α=1.0, pool=10, depth=10, candidates=10)`.
- Hybrid+CE `(α=1.0, pool=20, depth=20, candidates=20)`.
- FTS-only with bounded source-aware neighbor expansion.

Candidate-20 is a private evaluation adapter only, with tests proving no public API surface changed. If it wins, public configurable candidate breadth becomes an explicit Phase D product-escalation candidate.

Every configuration receives M1, M2, M4 proxy, M6, and M7 receipts. Prune M1 failures and Fast-profile latency failures; rank survivors by M1 then M2. Test an alternate embedder only when hybrid passes M1 and improves M2 by at least 0.01 over same-chunking FTS.

## M3/M4 shortlist and A0 parity

Score at most canonical A0, best Fast, and best Quality-GPU configurations, deduplicated.

Before deciding whether A0 needs paid rerun:

1. Run canonical A0 under the new façade in predict-only mode.
2. Compare its normalized, ordered top-10 retrieved content fingerprints per question with the historical external output.
3. Reuse historical A0 M3 only if every question matches and the answerer/judge model, prompt, scorer version, and scoring contract are pinned-identical.
4. Otherwise run fresh A0 M3 under the same scorer invocation as the two winners.

Keep the old 1007/1540 result as historical evidence either way. Compute actual temporal M4 only for these M3-scored configurations.

## Verification and follow-on

- TDD: provenance round-trip and ambiguity rejection; private candidate-20 isolation; deterministic timing; external-only artifacts; scorer resume/retry/cost; M4 proxy versus M4 labeling; A0 fingerprint-parity logic; δ bound behavior.
- Execute a small fixed-subset dry run, validate all receipt/index/artifact contracts, then run the full grid.
- Publish the capability report and scorecard/experiments-ledger updates after Phase B completes.
- Run Phase C Mem0 comparison only against selected Phase B winner(s), requiring paired 95% CI lower bound ≥ 0. Phase D begins only if the closed grid remains insufficient.
