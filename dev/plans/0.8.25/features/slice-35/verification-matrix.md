---
title: 0.8.25 Slice 35 verification matrix
status: GREEN
date: 2026-09-05
---

# Slice 35 verification matrix

## Acceptance coverage

| Acceptance | Executable evidence | Result |
|---|---|---|
| S35-AC1 closed eligibility vocabulary | `slice35_eligibility_contract`, `slice35_filter_grammar`, Python and TypeScript contract suites | Pass |
| S35-AC2 eligibility before caps | `slice35_eligibility_pretruncation`, `slice35_remaining_arm_matrix`, `slice35_graph_frontier_pretruncation`, `slice40_filter_unification`, `pr_g10_filtered_knn`, `slice15e_prekn_filterable` | Pass |
| S35-AC3 all-arm meaning | body, edge, property, vector, graph-seed, and graph-frontier real-database matrix; vector lifecycle state declares whole-arm fallback | Pass |
| S35-AC4 frozen context | `slice35_frozen_read`, schema `step31_frozen_reads`, normative `slice35_frozen_context_v1.json` | Pass |
| S35-AC5 snapshot safety | `slice35_frozen_read_races`, `slice35_after_validation_races` across lifecycle, erasure, dependency, projection, and rebuild state | Pass |
| S35-AC6 parity and bounded cost | PyO3, N-API, full TypeScript, installed-wheel/native-package smoke, CUDA, combined features, and v3 performance receipt | Pass on available platforms |

Schema trigger coverage executes insert/update/delete against all 14
authoritative tables and verifies the normalized 42-trigger set. A closed
source manifest also classifies every production FTS5/vec0 mutation site and
its real-row transaction coupling. The frozen codec fixture pins the canonical
context, registry and serving digests, payload, and token.

## Performance evidence

The immutable v1 and v2 receipts remain failures. After the bounded reader-
request correction, the independently preregistered v3 comparison passed the
three-percent 95% upper-bound policy:

| Receipt | Candidate | p50 upper | p95 upper | Measurement outcome | Classification status |
|---|---|---:|---:|---|---|
| `scale-02-slice35-20260905T0019Z-3756fde2` | `d49d445d` | 2.426% | 3.827% | Fail | Quarantined: false operation label |
| `scale-02-slice35-20260905T0031Z-864f6c61` | `d49d445d` | 2.448% | 30.545% | Fail | Quarantined: false operation label |
| `scale-02-slice35-20260905T0047Z-a934bd34` | `1dfe0a16` | 1.656% | 1.983% | Pass | Quarantined: false operation label |
| `scale-02-slice35-20260905T0231Z-cb9bad5b` | `0aff1cb0` | 0.852% | 1.422% | Pass | Quarantined: false operation label |
| `scale-02-slice35-20260905T0347Z-659c38ea` | `0aff1cb0` | 1.243% | 2.536% | Pass | Valid, superseded by final rerun |
| `scale-02-slice35-20260905T0404Z-659c38ea` | `0aff1cb0` | 1.623% | 2.121% | Pass | Authoritative |

All treatments used the pinned Slice 30 baseline
`b2bfb1f318f58041144acb2356a6a4c9624068b9`, fresh databases, identical 10k
input and query order, five repetitions per current campaign, 100 warm-ups,
1,000 measured searches, and zero errors/timeouts. The claim is limited to
legacy search non-regression. Frozen mint/consume overhead remains advisory.

The final row is authoritative. It binds the measured executable/native
candidate after all runtime implementation-review corrections and embeds its
measurement plan before execution. Its runner publishes a valid classification
sidecar with exact 5,500-call witnesses for both `Engine.search_text_only`
arms. Release source also contains the later type-stub-only `67f708fb`; that
public contract correction changes no measured executable implementation byte
and therefore does not require a benchmark rerun. The separate bulk-ingest
slowdown is retained as Slice 75 work; it is neither hidden nor used to
invalidate the narrower read claim.

## Platform and package routes

- Rust focused Engine/schema tests: pass.
- PyO3 library: 11/11 pass; N-API library: 10/10 pass.
- Full TypeScript source suite: 396/397 before the final typed-error correction;
  the corrected three-file frozen suite passes 3/3. The final repository gate
  passes 103/103 suites with zero skipped or excluded.
- CUDA dense-eligibility route on RTX 3090 GPU 0: pass with CUDA 12.6.
- Applicable serial combined-feature Engine suite: pass with `operator`,
  `test-hooks`, `default-embedder`, and `default-reranker`.
- Fresh wheel and offline npm/native smoke: pass, including exact normative
  token reproduction through both installed SDKs; artifacts are stored outside
  the checkout under `data/performance-benchmarking/scale-02/runtime/slice35/`.
- Windows runtime: unavailable; no Windows execution result is claimed.

The final independent implementation review passes with no actionable
P1/P2/material P3 finding. Independent verification reproduced the FIX-7 RED
and GREEN states, resolved the receipt's hashes and git blobs, recomputed its
statistics, and identified the type-stub wording correction recorded above.
