# Initial performance-benchmark population results — 2026-08-11

## Outcome

The zero-cost IR-C FTS characterization completed. The original linked
performance artifact is retained as typed-invalid evidence. After repairing
only the evaluator provenance bridge, a second run over the same frozen
quality workload produced two complete, provenance-qualified performance
cells. It is a one-repetition descriptive baseline, not a latency, QPS, or
competitor claim.

## Candidate and inputs

- Candidate: local instrumentation checkout `bf2d88a0b5df`
  (`v0.8.22-81-gbf2d88a0`); this is not a registry release label.
- Corpus: 10,506 documents pinned by
  `/tmp/fathomdb-0.8.24-instrumentation/tests/corpus/snapshot.json`.
- Gold: IR-C, 4,597 queries; gold SHA-256
  `4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb`;
  qrels version `ir-c-reused-v2`.
- Scenario: CPU FTS-only `Engine.search_text_only`, explicit `limit=10`, no
  embedder, evidence recall at 5 and 10, and no paid answer model.

## Quality result

Quality run: `earp-initial-population-fts-20260811T2056Z-eba4a4b6`

- Verdict: `complete`; 10,506 documents ingested and 4,597 retrievals.
- Strict/graded evidence recall (4,472 applicable queries): 0.6478 at 5 and
  0.6975 at 10.
- Negative abstention: 1 of 125 (0.008).
- Supporting coverage and nDCG are explicitly not applicable to this gold.

The uncommitted raw artifact is
`experiments/runs/earp-initial-population-fts-20260811T2056Z-eba4a4b6`.
Its result digest is
`adee28bea5bba6cfeac7ba38607fdfcaf45150c299e018820e4894b4309cd312`;
its immutable workload-manifest digest is
`6032607c426804c7280d4b780a03176f3ba9eea4bfbed48d986358194f517420`.

## Linked performance result

The original run,
`earp-characterization-performance-20260811T2101Z-d2d2622d`, is retained:
both cells are `invalid` with code `provenance_unavailable`. Its bridge did not
capture the command, device, fixture identities, or lockfile digest. Do not
quote or compare its raw timing samples. Its performance-evidence digest is
`29e6c1366487c0f9e5d1647ec787f4488042d731d0e3ee253a5550a81bcf832e`.

The repaired run is
`earp-characterization-performance-20260811T2152Z-a6321db9`. It used the
same immutable quality-manifest digest, candidate SHA, IR-C gold, CPU device,
FTS-only call, `limit=10`, one repetition, and two predeclared treatments.
Both cells are `complete`, with a command, device, fixtures, and lockfile
digest present; no provenance field is unavailable.

| Treatment | Open | Write | Full 4,597-query pass |
| --- | ---: | ---: | ---: |
| `fresh_store` | 32.352 ms | 2,096.115 ms | 212,314.715 ms |
| `fresh_store_warm_query` | 33.037 ms | 2,135.132 ms | 214,210.583 ms |

The uncommitted raw artifact is
`experiments/runs/earp-characterization-performance-20260811T2152Z-a6321db9`.
The execution checkout is marked `clean: false` because the bridge repair was
local; the linked quality run was clean and the native engine wheel, frozen
input manifest, and measurement configuration were unchanged. This proves
protocol comparability to that quality run, not comparison to a prior timing
run or another system.

## Next action

Use this artifact only as FathomDB's one-run baseline. A competitor comparison
must first implement one of the shared-data plans in this directory's
benchmark register, freeze its own comparator revision and knob ledger, and
run both arms over exactly the same raw input and question IDs.
