# TC-5 — eu7 grown-corpus characterization and advisory envelope

## Purpose and authority

This is the 0.8.23 **soft/advisory** F-17 deliverable: characterize eu7
ANN-fidelity on the grown **18,472-document** corpus and give the HITL an
evidence-backed advisory operating envelope. It is the explicit carried work
`TC-5` in the program sequencing record.

It is not an automatic relaxation of the 0.90 floor, a release gate, an ANN
change, an IR/evidence-recall result, or a claim about latency at a larger
scale. Any change to the shipped floor remains a separate explicit HITL
ruling after this result exists.

The eu7 SUT remains the existing pre-fusion vector stage: 1-bit candidate
search with `K=192`, followed by exact-f32 reranking, measured against the
same-model exact-f32 top-10 ground truth. The RRF-fused result remains
report-only. This measures ANN/quantization fidelity, not retrieval quality.

## Why a new runner mode is required

The current `eu7_real_corpus_ac.rs` is not yet a valid TC-5 runner:

- `load_subset_or_skip(usize::MAX)` consumes every JSONL row it finds, while
  the local raw directory currently has 23,306 rows rather than a
  manifest-proven 18,472-document snapshot.
- `haystack_bodies` pads above the loaded real corpus with synthetic
  distractors. Raising `EU7_N_VALUES` alone would therefore not establish an
  all-real 18,472-document result.
- The current ignored test asserts the historical `CURRENT_FLOOR=0.90` and
  overwrites `dev/plans/runs/eu7-latest-measurements.json`. Both are wrong for
  a report-only re-baseline and would risk replacing historical evidence.

TC-5 first adds a test-only characterization path; it does not alter the
legacy gate or silently repurpose its output.

## Frozen measurement contract

| Field | TC-5 setting |
| --- | --- |
| Primary arm | Exactly 18,472 manifest-selected, real documents; zero synthetic documents |
| Supporting bridge arm | Deterministic 7,667-document subset of that same manifest, report-only |
| Backend | CPU, same-backend as the 0.896 / CI-hi 0.925 baseline |
| Embedder | `fathomdb-bge-small-en-v1.5`, identity and asset hash recorded |
| SUT / GT | Pre-fusion 1-bit + f32-rerank vector stage / exact-f32 same-model top-10 |
| Candidate breadth | `K=192`, unchanged |
| Query selection | Existing deterministic synthesis, query-select seed `0x0E77C0125E1EC7` |
| Queries / bootstrap | 100 queries; 1,000 resamples, seed `0x0E77B007574A9` |
| Floor observation | Report point estimate, bootstrap CI, sigma, and the historical 0.90 predicate; do not assert it |

The CPU run is deliberate. The standing GPU policy treats a fidelity gate as
the narrow same-backend CPU repeatability exception: the observed GPU `N=7667`
result is cross-backend evidence, not a TC-5 regression comparator. GPU may
be used for corpus preparation or diagnostics after CUDA is verified, but it
cannot substitute for the primary measurement.

## Phase 0 — corpus and execution preflight

1. Produce an immutable, content-free corpus manifest for the candidate
   18,472 documents. It must pin source artifact hashes, normalized document
   IDs, deterministic order, inclusion/deduplication policy, total count, and
   the exact deterministic 7,667-document bridge subset. The raw corpus stays
   external and ignored.
2. Stop if the manifest does not resolve to exactly 18,472 unique usable
   documents. Do not compensate by taking all 23,306 rows, truncating each
   source, adding synthetic distractors, or changing the target count.
3. Pin source commit, Cargo lock hash, Rust/compiler versions, CPU identity,
   OS, model asset hash, engine feature set, and the `FATHOMDB_EMBED_DEVICE=cpu`
   setting. Confirm the model is cached before the long run.
4. Allocate an external campaign root such as
   `~/.local/share/fathomdb-experiments/eu7-tc5-20260814/` (mode 0700) for
   raw logs, per-query values, generated database, and full JSON output. Give
   every artifact a logical identifier and SHA-256. Git receives only a safe
   receipt and aggregate report—never corpus text, raw paths, or raw payloads.
5. Reserve the host for the long CPU run. Do not run competing embedding,
   benchmark, or CUDA workloads while it is seeding and measuring.

## Phase 1 — runner changes, test-first

Write failing tests before implementation for all of the following:

1. A manifest-backed loader accepts exactly the selected 18,472 real document
   IDs in canonical order and rejects a count mismatch, duplicate ID, missing
   ID, malformed row, or any synthetic document.
2. The bridge arm is a documented subset of the same manifest, not the old
   7,667-document corpus and not a per-source truncation accident.
3. Characterization mode requires an explicit external corpus location and
   external output directory; it never writes
   `dev/plans/runs/eu7-latest-measurements.json`.
4. The mode emits the complete frozen configuration, corpus-manifest hash,
   actual real-document count, no-padding assertion, fixed seeds, and model
   identity. It fails closed on any incomplete provenance.
5. Characterization mode reports the historical 0.90 one-sided-CI predicate
   but does not assert it. The existing gate mode retains its present
   `CURRENT_FLOOR` assertion unchanged.

Implement these as test-only helpers or an explicitly named ignored
characterization test. Keep the ordinary eu7 AC-075 path and its historical
output contract intact. Add a safe receipt projection that references only
logical external artifacts plus hashes.

## Phase 2 — execution

Run a small manifest/loader smoke test first; it proves zero padding and the
external-output contract without spending a full measurement. Then run the
two frozen all-real arms in one CPU characterization invocation:

```text
N = 7,667  supporting bridge, report-only
N = 18,472 primary grown-corpus result, advisory-envelope input
```

Use the existing real-embedder feature set and ignored-test discipline, with
`AGENT_LONG=1`, cached model assets, the CPU device setting, and release-mode
build pinned in the receipt. The runner must report completion counts and
write the external result before rendering an advisory verdict. A host failure,
partial query set, failed provenance check, or incomplete bootstrap is
**inconclusive**, not a lower floor.

The historical 0.896 CPU / CI-hi 0.925 figure is context only: it came from a
different corpus snapshot. The bridge arm helps characterize the new snapshot
at historical N, but it does not turn the comparison into a product regression
claim.

## Phase 3 — analysis and advisory envelope

For each arm, report vector-stage recall@10, 95% bootstrap CI, bootstrap
sigma, the report-only fused delta, the observed historical 0.90 predicate,
and `R - 2σ` (also rounded down to 0.01). The latter is a candidate advisory
lower bound, never an automatically adopted threshold.

The final advisory envelope must name its scope explicitly:

- corpus manifest hash and exactly 18,472 real documents;
- CPU backend, model identity/assets, mean-centering behavior, `K=192`,
  ground-truth method, query construction, seeds, and resample count;
- the measurement commit and environment receipt; and
- exclusions: GPU backend, other corpora or corpus revisions, larger `N`,
  latency/SLO claims, fused-search relevance, and IR/evidence recall.

Decision rubric for the HITL:

1. If the primary arm's CI-hi clears 0.90, recommend retaining 0.90 as the
   advisory reference for the manifested corpus; still publish the bounded
   envelope.
2. If it does not clear, present the observed result and candidate lower bound
   with no self-applied floor change. The HITL chooses between retaining 0.90
   as an unmet product target, adopting a lower *soft* advisory value, or
   commissioning diagnosis/remediation.
3. If provenance, completeness, or repeatability fails, make no numerical
   recommendation and repeat only after the defect is resolved.

## Close-out

After an actual run, append a safe factual summary to the experiments ledger,
index its receipt, and update TC-5 through the ledger tool with the HITL's
ruling. Do not change `CURRENT_FLOOR`, publish a stated/hard scale promise, or
convert the advisory result into a release condition without a later approved
decision and the corresponding test/contract update.
