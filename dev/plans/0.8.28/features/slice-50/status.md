---
title: FathomDB 0.8.28 Slice 50 — status
status: implementation reviewed and held-drain qualification passed; final verification pending
target_release: 0.8.28
observed_on: 2026-11-01
---

# Slice 50 status

The remaining scan Unit S is implemented on baseline
`630af0bfcb0c5779d436ddbd5ffe3a0e876574b5`. Both scan arms have an ordered
256-row bound; predicates, page loops, generation joins and rewind behavior
are unchanged. The [plan](plan.md) records changes since the original draft
and the [requirements](requirements.md) define the active acceptance criteria.

The approved provider subunit was already complete at the baseline, with
independent Astra review and full verification coverage plus scoped repairs.
Its [implementation record](built-in-provider-implementation-record.md) remains
authoritative. Historical D50-1/D50-2 refused-handle options were not selected;
actual verification refusal now fails open without returning an Engine.
Their old proposals are not remaining slice work. Cache behavior, pooling,
provider policy, schema and public production APIs are unchanged by Unit S.

## Implementation and review

| Work | Evidence |
| --- | --- |
| Reconciled plan, requirements, design and test plan | `f682f242e`; independent Astra design APPROVE, no P1/P2 |
| Unchanged SQL extraction and baseline guards | `2e4714ff3`; 64 generated every-cursor cases and three ordering layouts |
| Scan RED | `4d1134da6`; missing bounded arm co-routine; VM-step ratios 7.7782 scheduler, 7.8626 candidate |
| Scan GREEN | `65b8fd281`; five focused tests pass; both VM-step ratios exactly 1.0 |
| Scaling probe and driver | `06dd97130`; deterministic corpus and embedder, paired builds/runs and fixed decision rules |
| Digest review repair | RED `0918245f2`, GREEN `0adbebea2`; unmatched real-SQLite vectors/mappings detected by independent whole-table hashing |
| Independent code review | Astra APPROVE at `0adbebea2c228fe1e232b094a37055a6c0170471`; no remaining P1/P2 |
| Paired build correction | `dd1d05729`; refresh exported source timestamps to avoid stale shared-target artifacts; Astra reviewed, refresh placed under build lock |

The scan property guard passes against the baseline and optimized SQL.
Mutating the candidate arm limit to 1 fails randomized equivalence; reversing
its order fails the 600-row arm-dominant ordering guard. Production source
was restored and the deliberate mutation regression artifact removed.

Both optimized pages cost the same VM steps at 2,000 nodes/500 edges and
16,000 nodes/4,000 edges: scheduler 26,978; candidate 24,314. The bound is on
accepted arm output and the outer sort, not on rejected index entries.
Ordinary canonical nodes remain projection members when superseded; edges
filter supersession and expiry. Tests preserve that existing distinction.

The measurement tool's seven unit tests pass. Its actual single-thread probe
smoke passes at 100 rows. Complete-table digest tests both pass. Module
boundaries and scoped Markdown validation pass. The initial paired-build
attempt exposed Cargo reuse from archived source mtimes; it was discarded
before timing. Corrected base/head binaries have different hashes and each
was compiled from the intended exported source.

## Scaling qualification

Held-drain acceptance **PASS** on windchill3 (24 logical CPUs), paired base
`630af0bfc` and head `0adbebea2`, format 34, SQLite 3.54.1, default cache,
768-d deterministic vectors and two interleaved repeats:

| Vectors | Base median drain | Head median drain | Base µs/vector | Head µs/vector |
| --- | --- | --- | --- | --- |
| 10,000 | 3.321 s | 3.264 s | 332.13 | 326.43 |
| 30,000 | 10.565 s | 10.117 s | 352.16 | 337.23 |
| 100,000 | 46.724 s | 33.707 s | 467.24 | 337.07 |

- AC50-08: head 100k:10k cost/vector ratio **1.033 ≤ 1.3**; head:base
  10k median ratio **0.983 ≤ 1.05**. Base scaling ratio was 1.407.
- Reported 100k head:base paired ratios: 0.716 and 0.727 (27–28% faster).
- AC50-07: all base/head/repeat full-table digests match at every N.
- The second base 100k attempt had load peak 2.573 and was repeated once
  under the predeclared rule. Its replacement peak was 2.015, with matching
  digest; no selected held cell remains flagged. The first attempt remains
  in the raw log and does not enter the medians.
- Reported-only overlap: base write/drain 42.186/0.795 s; head
  41.639/0.150 s. Both reach 100k vectors and the same digest. The initial
  head overlap probe exited 101; its captured diagnostics were lost by the
  driver. One scoped repeat passed. This remains an unexplained one-off
  failure, not a claimed clean paired run. A focused RED/GREEN tool test now
  ensures subprocess failure stdout/stderr is printed for future diagnosis.
  The completed held-drain acceptance results are independent of this cell.

Tools and rerun instructions: [tools/README.md](tools/README.md). Raw logs,
binaries and results are outside the repository at
`/tmp/slice50-scaling-20261101-v2`; this compact record is tracked. No timing
threshold was changed after measurement, and no AC-013 search-latency or
universal sparse-backlog scaling claim is made.

## Final verification and landing

Pending independent verification on the final clean candidate, then landing
on `dev/0.8.28` and cleanup of the existing slice branch/worktree. No full-gate
pass or Slice 50 completion is claimed yet. Slices 60–90, release final
verification, publication and the separately deferred provider-free rebuild
defect are outside this work.

## Follow-on planning finding

Slice 50 preserves the existing node/edge supersession asymmetry. Astra
review confirmed a Slice 80 planning gap: retaining existing edge vectors
alone cannot serve an edge superseded before embedding. The master plan now
requires a consistent historical dense-membership/completeness contract,
including pre-drain, post-drain, in-flight and reopen/rebuild cases and
truthful handling of skipped terminals or unavailable projections. Default
search exclusions stay fixed; historical search requires `include_superseded`.
This clarification changes no Slice 50 production predicate or storage policy.

## Historical investigation

The original phase-1 plan was `ac83688a1` on `544bf74f8`. The
[refusal characterization](../../../../experiments/slice50-refusal-baseline/README.md)
and [fresh-open study](../../../../experiments/fresh-open-verification/findings.md)
led to the narrowed provider contract approved by owner `seq-359`. The
[accepted ADR](../../../../adr/ADR-0.8.28-vector-compatibility-open-policy.md)
and provider implementation record preserve the resulting contract and
verification history.
